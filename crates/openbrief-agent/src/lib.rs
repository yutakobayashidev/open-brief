//! Bounded ACP runtime used by the `OpenBrief` desktop.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::schema::v1::{
    AuthenticateRequest, ContentBlock, InitializeRequest, McpServer, McpServerStdio,
    NewSessionRequest, PromptRequest, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, SelectedPermissionOutcome, SessionId, SessionNotification,
    SessionUpdate, TextContent,
};
use agent_client_protocol::{
    AcpAgent, AcpAgentConfig, Agent, ConnectionTo, on_receive_notification, on_receive_request,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConfig {
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub working_directory: PathBuf,
    pub mcp_server: Option<McpServerConfig>,
    pub startup_timeout: Duration,
    pub turn_timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServerConfig {
    pub executable: PathBuf,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthMethodInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeState {
    Stopped,
    Starting,
    Ready,
    Busy,
    Stopping,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub generation: Uuid,
    pub session_id: Option<String>,
    pub turn_id: Option<Uuid>,
    pub sequence: u64,
    pub event: AgentEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    RuntimeStateChanged {
        state: RuntimeState,
    },
    TextDelta {
        text: String,
    },
    ToolCall {
        title: String,
        status: String,
    },
    PermissionRequested {
        request_id: String,
        title: String,
        options: Vec<PermissionOption>,
    },
    TurnFinished,
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionOption {
    pub id: String,
    pub label: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("agent executable must be an explicit path")]
    ExecutablePathRequired,
    #[error("agent working directory must be absolute")]
    WorkingDirectoryMustBeAbsolute,
    #[error("MCP executable must be an explicit path")]
    McpExecutablePathRequired,
    #[error("agent runtime did not become ready")]
    StartupTimeout,
    #[error("agent runtime stopped before becoming ready")]
    StartupFailed,
    #[error("agent session is unavailable: {message}")]
    SessionUnavailable {
        message: String,
        auth_methods: Vec<AuthMethodInfo>,
    },
    #[error("agent does not advertise authentication method {0}")]
    AuthMethodUnavailable(String),
    #[error("agent runtime command channel is closed")]
    RuntimeStopped,
    #[error("agent turn timed out")]
    TurnTimeout,
    #[error("ACP error: {0}")]
    Acp(#[from] agent_client_protocol::Error),
}

impl AgentConfig {
    /// Validates that process paths are explicit and the working directory is absolute.
    ///
    /// # Errors
    ///
    /// Returns an error when an executable path or working directory is not absolute.
    pub fn validate(&self) -> Result<(), AgentError> {
        if !self.executable.is_absolute() {
            return Err(AgentError::ExecutablePathRequired);
        }
        if !self.working_directory.is_absolute() {
            return Err(AgentError::WorkingDirectoryMustBeAbsolute);
        }
        if self
            .mcp_server
            .as_ref()
            .is_some_and(|server| !server.executable.is_absolute())
        {
            return Err(AgentError::McpExecutablePathRequired);
        }
        Ok(())
    }
}

pub type EventSink = Arc<dyn Fn(EventEnvelope) + Send + Sync>;

pub struct AgentRuntime {
    generation: Uuid,
    command_tx: mpsc::Sender<RuntimeCommand>,
    cancel_tx: watch::Sender<u64>,
    task: JoinHandle<()>,
}

impl AgentRuntime {
    /// Starts the ACP process and waits for it to initialize.
    ///
    /// # Errors
    ///
    /// Returns an error when configuration is invalid, ACP initialization fails, or
    /// the startup timeout expires.
    pub async fn start(config: AgentConfig, sink: EventSink) -> Result<Self, AgentError> {
        config.validate()?;
        let generation = Uuid::new_v4();
        let emitter = Arc::new(EventEmitter::new(generation, sink));
        emitter.emit(AgentEvent::RuntimeStateChanged {
            state: RuntimeState::Starting,
        });

        let (command_tx, command_rx) = mpsc::channel(4);
        let (cancel_tx, cancel_rx) = watch::channel(0);
        let (ready_tx, ready_rx) = oneshot::channel();
        let startup_timeout = config.startup_timeout;
        let task_emitter = emitter.clone();

        let task = tokio::spawn(async move {
            if let Err(error) = run_connection(
                config,
                command_rx,
                cancel_rx,
                ready_tx,
                task_emitter.clone(),
            )
            .await
            {
                task_emitter.emit(AgentEvent::RuntimeStateChanged {
                    state: RuntimeState::Failed,
                });
                task_emitter.emit(AgentEvent::Error {
                    message: error.to_string(),
                });
            }
        });

        match tokio::time::timeout(startup_timeout, ready_rx).await {
            Ok(Ok(Ok(()))) => Ok(Self {
                generation,
                command_tx,
                cancel_tx,
                task,
            }),
            Ok(Ok(Err(failure))) => {
                task.abort();
                Err(AgentError::SessionUnavailable {
                    message: failure.message,
                    auth_methods: failure.auth_methods,
                })
            }
            Ok(Err(_)) => {
                task.abort();
                Err(AgentError::StartupFailed)
            }
            Err(_) => {
                task.abort();
                Err(AgentError::StartupTimeout)
            }
        }
    }

    #[must_use]
    pub fn generation(&self) -> Uuid {
        self.generation
    }

    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.task.is_finished()
    }

    /// Queues a prompt and returns its local turn identifier.
    ///
    /// # Errors
    ///
    /// Returns [`AgentError::RuntimeStopped`] when the runtime is no longer accepting commands.
    pub async fn prompt(&self, text: String) -> Result<Uuid, AgentError> {
        let turn_id = Uuid::new_v4();
        self.command_tx
            .send(RuntimeCommand::Prompt { turn_id, text })
            .await
            .map_err(|_| AgentError::RuntimeStopped)?;
        Ok(turn_id)
    }

    pub fn cancel(&self) {
        let next = (*self.cancel_tx.borrow()).saturating_add(1);
        let _ = self.cancel_tx.send(next);
    }

    pub async fn stop(self) {
        let _ = self.command_tx.send(RuntimeCommand::Stop).await;
        let _ = tokio::time::timeout(Duration::from_secs(2), self.task).await;
    }
}

/// Runs one ACP authentication method and exits after the adapter reports completion.
///
/// # Errors
///
/// Returns an error when the executable is not explicit, the method is not advertised,
/// the adapter fails, or the authentication timeout expires.
pub async fn authenticate(
    executable: PathBuf,
    args: Vec<String>,
    method_id: String,
    timeout: Duration,
) -> Result<(), AgentError> {
    if !executable.is_absolute() {
        return Err(AgentError::ExecutablePathRequired);
    }
    let requested_method = method_id.clone();
    let operation = async move {
        let agent = AcpAgent::new(AcpAgentConfig::new(executable).args(args));
        agent_client_protocol::Client
            .builder()
            .connect_with(agent, |connection: ConnectionTo<Agent>| async move {
                let initialize = connection
                    .send_request(InitializeRequest::new(ProtocolVersion::V1))
                    .block_task()
                    .await?;
                if !initialize
                    .auth_methods
                    .iter()
                    .any(|method| method.id().to_string() == requested_method)
                {
                    return Err(agent_client_protocol::Error::new(
                        -32602,
                        AgentError::AuthMethodUnavailable(requested_method.clone()).to_string(),
                    ));
                }
                connection
                    .send_request(AuthenticateRequest::new(requested_method))
                    .block_task()
                    .await?;
                Ok(())
            })
            .await
            .map_err(AgentError::from)
    };
    tokio::time::timeout(timeout, operation)
        .await
        .map_err(|_| AgentError::TurnTimeout)?
}

enum RuntimeCommand {
    Prompt { turn_id: Uuid, text: String },
    Stop,
}

struct EventContext {
    session_id: Option<String>,
    turn_id: Option<Uuid>,
}

struct EventEmitter {
    generation: Uuid,
    sequence: AtomicU64,
    context: std::sync::Mutex<EventContext>,
    sink: EventSink,
}

struct StartupFailure {
    message: String,
    auth_methods: Vec<AuthMethodInfo>,
}

impl EventEmitter {
    fn new(generation: Uuid, sink: EventSink) -> Self {
        Self {
            generation,
            sequence: AtomicU64::new(0),
            context: std::sync::Mutex::new(EventContext {
                session_id: None,
                turn_id: None,
            }),
            sink,
        }
    }

    fn set_session(&self, session_id: String) {
        self.context
            .lock()
            .expect("event context poisoned")
            .session_id = Some(session_id);
    }

    fn set_turn(&self, turn_id: Option<Uuid>) {
        self.context.lock().expect("event context poisoned").turn_id = turn_id;
    }

    fn emit(&self, event: AgentEvent) {
        let context = self.context.lock().expect("event context poisoned");
        (self.sink)(EventEnvelope {
            generation: self.generation,
            session_id: context.session_id.clone(),
            turn_id: context.turn_id,
            sequence: self.sequence.fetch_add(1, Ordering::Relaxed),
            event,
        });
    }
}

async fn run_connection(
    config: AgentConfig,
    mut command_rx: mpsc::Receiver<RuntimeCommand>,
    mut cancel_rx: watch::Receiver<u64>,
    ready_tx: oneshot::Sender<Result<(), StartupFailure>>,
    emitter: Arc<EventEmitter>,
) -> Result<(), AgentError> {
    let mut process = AcpAgentConfig::new(&config.executable).args(config.args.clone());
    process = process.env("INITIAL_AGENT_MODE", "read-only");
    let agent = AcpAgent::new(process);

    let notification_emitter = emitter.clone();
    let permission_emitter = emitter.clone();
    agent_client_protocol::Client
        .builder()
        .on_receive_notification(
            async move |notification: SessionNotification, _connection| {
                emit_session_update(&notification_emitter, notification.update);
                Ok(())
            },
            on_receive_notification!(),
        )
        .on_receive_request(
            async move |request: RequestPermissionRequest, responder, _connection| {
                let response = permission_response(&permission_emitter, &request);
                responder.respond(response)
            },
            on_receive_request!(),
        )
        .connect_with(agent, |connection: ConnectionTo<Agent>| async move {
            let Some(session_id) =
                initialize_session(&connection, &config, ready_tx, &emitter).await?
            else {
                return Ok(());
            };

            while let Some(command) = command_rx.recv().await {
                match command {
                    RuntimeCommand::Prompt { turn_id, text } => {
                        emitter.set_turn(Some(turn_id));
                        emitter.emit(AgentEvent::RuntimeStateChanged {
                            state: RuntimeState::Busy,
                        });
                        let request = connection.send_request(PromptRequest::new(
                            session_id.clone(),
                            vec![ContentBlock::Text(TextContent::new(text))],
                        ));
                        let prompt = request.block_task();
                        tokio::pin!(prompt);
                        let result = tokio::select! {
                            result = tokio::time::timeout(config.turn_timeout, &mut prompt) => {
                                match result {
                                    Ok(result) => result.map(|_| ()).map_err(AgentError::from),
                                    Err(_) => Err(AgentError::TurnTimeout),
                                }
                            }
                            changed = cancel_rx.changed() => {
                                if changed.is_ok() {
                                    let _ = connection.send_notification(
                                        agent_client_protocol::schema::v1::CancelNotification::new(
                                            session_id.clone(),
                                        ),
                                    );
                                }
                                prompt.await.map(|_| ()).map_err(AgentError::from)
                            }
                        };
                        match result {
                            Ok(()) => emitter.emit(AgentEvent::TurnFinished),
                            Err(error) => emitter.emit(AgentEvent::Error {
                                message: error.to_string(),
                            }),
                        }
                        emitter.set_turn(None);
                        emitter.emit(AgentEvent::RuntimeStateChanged {
                            state: RuntimeState::Ready,
                        });
                    }
                    RuntimeCommand::Stop => break,
                }
            }
            emitter.emit(AgentEvent::RuntimeStateChanged {
                state: RuntimeState::Stopped,
            });
            Ok(())
        })
        .await?;
    Ok(())
}

async fn initialize_session(
    connection: &ConnectionTo<Agent>,
    config: &AgentConfig,
    ready_tx: oneshot::Sender<Result<(), StartupFailure>>,
    emitter: &EventEmitter,
) -> Result<Option<SessionId>, agent_client_protocol::Error> {
    let initialize = connection
        .send_request(InitializeRequest::new(ProtocolVersion::V1))
        .block_task()
        .await?;
    let auth_methods = initialize
        .auth_methods
        .iter()
        .map(|method| AuthMethodInfo {
            id: method.id().to_string(),
            name: method.name().to_owned(),
        })
        .collect::<Vec<_>>();

    let mut request = NewSessionRequest::new(&config.working_directory);
    if let Some(server) = &config.mcp_server {
        request = request.mcp_servers(vec![McpServer::Stdio(
            McpServerStdio::new("openbrief", &server.executable).args(server.args.clone()),
        )]);
    }
    let session = match connection.send_request(request).block_task().await {
        Ok(session) => session,
        Err(error) => {
            let _ = ready_tx.send(Err(StartupFailure {
                message: error.to_string(),
                auth_methods,
            }));
            return Ok(None);
        }
    };
    let session_id = session.session_id;
    emitter.set_session(session_id.to_string());
    emitter.emit(AgentEvent::RuntimeStateChanged {
        state: RuntimeState::Ready,
    });
    let _ = ready_tx.send(Ok(()));
    Ok(Some(session_id))
}

fn emit_session_update(emitter: &EventEmitter, update: SessionUpdate) {
    match update {
        SessionUpdate::AgentMessageChunk(chunk) => {
            if let ContentBlock::Text(text) = chunk.content {
                emitter.emit(AgentEvent::TextDelta { text: text.text });
            }
        }
        SessionUpdate::ToolCall(call) => emitter.emit(AgentEvent::ToolCall {
            title: call.title,
            status: format!("{:?}", call.status).to_lowercase(),
        }),
        SessionUpdate::ToolCallUpdate(update) => emitter.emit(AgentEvent::ToolCall {
            title: update
                .fields
                .title
                .unwrap_or_else(|| update.tool_call_id.to_string()),
            status: update.fields.status.map_or_else(
                || "in_progress".to_owned(),
                |status| format!("{status:?}").to_lowercase(),
            ),
        }),
        _ => {}
    }
}

fn permission_response(
    emitter: &EventEmitter,
    request: &RequestPermissionRequest,
) -> RequestPermissionResponse {
    let is_proposal_tool = request
        .tool_call
        .fields
        .title
        .as_deref()
        .is_some_and(is_proposal_tool_title);
    let selected = request.options.iter().find(|option| {
        matches!(
            option.kind,
            agent_client_protocol::schema::v1::PermissionOptionKind::AllowOnce
        ) == is_proposal_tool
            && (is_proposal_tool
                || matches!(
                    option.kind,
                    agent_client_protocol::schema::v1::PermissionOptionKind::RejectOnce
                        | agent_client_protocol::schema::v1::PermissionOptionKind::RejectAlways
                ))
    });
    emitter.emit(AgentEvent::PermissionRequested {
        request_id: request.tool_call.tool_call_id.to_string(),
        title: request
            .tool_call
            .fields
            .title
            .clone()
            .unwrap_or_else(|| "Agent tool request".to_owned()),
        options: request
            .options
            .iter()
            .map(|option| PermissionOption {
                id: option.option_id.to_string(),
                label: option.name.clone(),
            })
            .collect(),
    });
    selected.map_or_else(
        || RequestPermissionResponse::new(RequestPermissionOutcome::Cancelled),
        |option| {
            RequestPermissionResponse::new(RequestPermissionOutcome::Selected(
                SelectedPermissionOutcome::new(option.option_id.clone()),
            ))
        },
    )
}

fn is_proposal_tool_title(title: &str) -> bool {
    title
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .any(|token| matches!(token, "brief_propose" | "triage_propose"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_explicit_paths() {
        let config = AgentConfig {
            executable: "codex-acp".into(),
            args: Vec::new(),
            working_directory: "/tmp".into(),
            mcp_server: None,
            startup_timeout: Duration::from_secs(1),
            turn_timeout: Duration::from_secs(1),
        };
        assert!(matches!(
            config.validate(),
            Err(AgentError::ExecutablePathRequired)
        ));
    }

    #[test]
    fn proposal_tool_matching_does_not_inspect_arbitrary_arguments() {
        assert!(is_proposal_tool_title("Call brief_propose"));
        assert!(is_proposal_tool_title("triage_propose"));
        assert!(!is_proposal_tool_title("shell"));
        assert!(!is_proposal_tool_title(
            "shell with argument saying brief_propose_extra"
        ));
    }
}
