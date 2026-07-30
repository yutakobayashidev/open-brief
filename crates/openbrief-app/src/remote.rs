use std::fs;
use std::net::SocketAddr;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{DefaultBodyLimit, Path as AxumPath, Query, Request, State};
use axum::http::header::{AUTHORIZATION, WWW_AUTHENTICATE};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use futures_util::{SinkExt, StreamExt};
use openbrief_protocol::{
    AgentStatus, ApiErrorBody, ApiErrorDetail, ApiResponse, RemoteSnapshot, StartTurnRequest,
    StartedTurn, TriageProposalView,
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::time::MissedTickBehavior;

use crate::agent_host::{
    apply_proposal, latest_unconfirmed_triage, load_brief, load_return_thread,
};
use crate::{AgentHost, AgentHostRequest, AgentHostResponse, EventJournal, RemoteSettings};

const BODY_LIMIT: usize = 64 * 1024;
const MAX_PROMPT_CHARS: usize = 4_000;
const READY_TIMEOUT: Duration = Duration::from_secs(5);
const EVENT_INTERVAL: Duration = Duration::from_millis(100);
const PING_INTERVAL_TICKS: u16 = 150;
const EVENT_SUBPROTOCOL: &str = "openbrief.events.v1";

pub(crate) struct RemoteServer {
    shutdown: Option<oneshot::Sender<()>>,
    thread: Option<thread::JoinHandle<()>>,
}

impl RemoteServer {
    pub(crate) fn start(
        settings: &RemoteSettings,
        database_file: PathBuf,
        agent: AgentHost,
        journal: EventJournal,
    ) -> Result<Option<Self>, RemoteError> {
        if !settings.enabled {
            return Ok(None);
        }
        let token_file = settings
            .token_file
            .as_deref()
            .ok_or(RemoteError::TokenPathRequired)?;
        let token = load_token(token_file)?;
        let bind = settings.bind;
        let stopping = Arc::new(AtomicBool::new(false));
        let state = RemoteState {
            token: Arc::from(token),
            database_file,
            agent,
            journal,
            stopping: stopping.clone(),
        };
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let handle = thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build();
            let Ok(runtime) = runtime else {
                let _ = ready_tx.send(Err("could not create remote API runtime".to_owned()));
                return;
            };
            runtime.block_on(async move {
                let listener = match TcpListener::bind(bind).await {
                    Ok(listener) => listener,
                    Err(error) => {
                        let _ = ready_tx.send(Err(error.to_string()));
                        return;
                    }
                };
                let _ = ready_tx.send(Ok(()));
                let app = router(state);
                let _ = axum::serve(listener, app)
                    .with_graceful_shutdown(async move {
                        let _ = shutdown_rx.await;
                        stopping.store(true, Ordering::Release);
                    })
                    .await;
            });
        });
        match ready_rx.recv_timeout(READY_TIMEOUT) {
            Ok(Ok(())) => Ok(Some(Self {
                shutdown: Some(shutdown_tx),
                thread: Some(handle),
            })),
            Ok(Err(message)) => {
                let _ = handle.join();
                Err(RemoteError::Bind { bind, message })
            }
            Err(_) => Err(RemoteError::StartupTimeout(bind)),
        }
    }

    pub(crate) fn stop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for RemoteServer {
    fn drop(&mut self) {
        self.stop();
    }
}

#[derive(Clone)]
struct RemoteState {
    token: Arc<str>,
    database_file: PathBuf,
    agent: AgentHost,
    journal: EventJournal,
    stopping: Arc<AtomicBool>,
}

fn router(state: RemoteState) -> Router {
    let protected = Router::new()
        .route("/v1/snapshot", get(snapshot))
        .route("/v1/agent-session", put(start_agent))
        .route("/v1/turns", post(start_turn))
        .route(
            "/v1/proposals/{proposal_id}/confirmations",
            post(confirm_proposal),
        )
        .route("/v1/events", get(events))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_token));
    Router::new()
        .route("/health", get(health))
        .merge(protected)
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
        .with_state(state)
}

async fn require_token(State(state): State<RemoteState>, request: Request, next: Next) -> Response {
    let expected = format!("Bearer {}", state.token);
    let authorized = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == expected);
    if authorized {
        next.run(request).await
    } else {
        ApiError::unauthorized().into_response()
    }
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn snapshot(
    State(state): State<RemoteState>,
) -> Result<Json<ApiResponse<RemoteSnapshot>>, ApiError> {
    let database_file = state.database_file.clone();
    let (brief, return_thread, pending_proposal) = tokio::task::spawn_blocking(move || {
        Ok::<_, String>((
            load_brief(&database_file)?,
            load_return_thread(&database_file)?,
            latest_unconfirmed_triage(&database_file)?,
        ))
    })
    .await
    .map_err(ApiError::task)?
    .map_err(ApiError::application)?;
    let agent = request_agent(state.agent.clone(), AgentHostRequest::Status)
        .await
        .and_then(expect_agent_status)?;
    Ok(Json(ApiResponse {
        data: RemoteSnapshot {
            brief,
            return_thread,
            pending_proposal,
            agent,
            next_sequence: state.journal.cursor(),
        },
    }))
}

async fn start_agent(
    State(state): State<RemoteState>,
) -> Result<Json<ApiResponse<AgentStatus>>, ApiError> {
    let status = request_agent(state.agent, AgentHostRequest::Start)
        .await
        .and_then(expect_agent_status)?;
    Ok(Json(ApiResponse { data: status }))
}

async fn start_turn(
    State(state): State<RemoteState>,
    Json(request): Json<StartTurnRequest>,
) -> Result<(StatusCode, Json<ApiResponse<StartedTurn>>), ApiError> {
    let text = request.text.trim();
    if text.is_empty() {
        return Err(ApiError::validation("text", "required"));
    }
    if text.chars().count() > MAX_PROMPT_CHARS {
        return Err(ApiError::validation("text", "too_long"));
    }
    let response = request_agent(
        state.agent,
        AgentHostRequest::Prompt {
            text: text.to_owned(),
        },
    )
    .await?;
    let AgentHostResponse::TurnStarted { id } = response else {
        return Err(ApiError::internal("agent returned an unexpected response"));
    };
    Ok((
        StatusCode::ACCEPTED,
        Json(ApiResponse {
            data: StartedTurn { id },
        }),
    ))
}

async fn confirm_proposal(
    State(state): State<RemoteState>,
    AxumPath(proposal_id): AxumPath<String>,
) -> Result<Json<ApiResponse<TriageProposalView>>, ApiError> {
    if proposal_id.trim().is_empty() || proposal_id.len() > 256 {
        return Err(ApiError::validation("proposal_id", "invalid"));
    }
    let database_file = state.database_file.clone();
    let journal = state.journal;
    let proposal = tokio::task::spawn_blocking(move || {
        let proposal = apply_proposal(&database_file, &proposal_id)?;
        journal.push(crate::DaemonEvent::ProposalApplied {
            proposal: proposal.clone(),
        });
        Ok::<_, String>(proposal)
    })
    .await
    .map_err(ApiError::task)?
    .map_err(ApiError::conflict)?;
    Ok(Json(ApiResponse { data: proposal }))
}

async fn events(
    State(state): State<RemoteState>,
    Query(query): Query<EventQuery>,
    websocket: WebSocketUpgrade,
) -> Response {
    let websocket = websocket.protocols([EVENT_SUBPROTOCOL]);
    if websocket.selected_protocol().is_none() {
        return ApiError::websocket_subprotocol().into_response();
    }
    websocket.on_upgrade(move |socket| event_stream(socket, state, query.after.unwrap_or(0)))
}

async fn event_stream(socket: WebSocket, state: RemoteState, mut cursor: u64) {
    let (mut output, mut input) = socket.split();
    let mut interval = tokio::time::interval(EVENT_INTERVAL);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut ticks = 0_u16;
    loop {
        tokio::select! {
            _ = interval.tick() => {
                if state.stopping.load(Ordering::Acquire) {
                    let _ = output.send(Message::Close(None)).await;
                    break;
                }
                let (events, next_sequence) = state.journal.after(cursor);
                cursor = next_sequence;
                for event in events {
                    let Ok(payload) = serde_json::to_string(&event) else {
                        continue;
                    };
                    if output.send(Message::Text(payload.into())).await.is_err() {
                        return;
                    }
                }
                ticks = ticks.saturating_add(1);
                if ticks >= PING_INTERVAL_TICKS {
                    ticks = 0;
                    if output.send(Message::Ping(Bytes::new())).await.is_err() {
                        return;
                    }
                }
            }
            message = input.next() => {
                match message {
                    Some(Ok(Message::Close(_)) | Err(_)) | None => break,
                    Some(Ok(_)) => {}
                }
            }
        }
    }
}

async fn request_agent(
    agent: AgentHost,
    request: AgentHostRequest,
) -> Result<AgentHostResponse, ApiError> {
    tokio::task::spawn_blocking(move || agent.request(request))
        .await
        .map_err(ApiError::task)?
        .map_err(|error| ApiError::application(error.to_string()))
}

fn expect_agent_status(response: AgentHostResponse) -> Result<AgentStatus, ApiError> {
    match response {
        AgentHostResponse::Status(status) => Ok(status),
        AgentHostResponse::TurnStarted { .. } | AgentHostResponse::Ok => {
            Err(ApiError::internal("agent returned an unexpected response"))
        }
    }
}

fn load_token(path: &Path) -> Result<String, RemoteError> {
    let metadata = fs::metadata(path).map_err(|source| RemoteError::TokenRead {
        path: path.to_owned(),
        source,
    })?;
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(RemoteError::TokenPermissions(path.to_owned()));
    }
    let token = fs::read_to_string(path)
        .map_err(|source| RemoteError::TokenRead {
            path: path.to_owned(),
            source,
        })?
        .trim()
        .to_owned();
    if !(32..=256).contains(&token.len()) {
        return Err(RemoteError::TokenLength);
    }
    Ok(token)
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Debug, Deserialize)]
struct EventQuery {
    after: Option<u64>,
}

struct ApiError {
    status: StatusCode,
    body: ApiErrorBody,
}

impl ApiError {
    fn unauthorized() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            body: ApiErrorBody {
                code: "unauthorized".to_owned(),
                message: "A valid OpenBrief device token is required.".to_owned(),
                retryable: false,
                details: Vec::new(),
            },
        }
    }

    fn validation(field: &'static str, reason: &'static str) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            body: ApiErrorBody {
                code: "invalid_request".to_owned(),
                message: "The request is invalid.".to_owned(),
                retryable: false,
                details: vec![ApiErrorDetail {
                    field: field.to_owned(),
                    reason: reason.to_owned(),
                }],
            },
        }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            body: ApiErrorBody {
                code: "state_conflict".to_owned(),
                message: message.into(),
                retryable: false,
                details: Vec::new(),
            },
        }
    }

    fn application(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            body: ApiErrorBody {
                code: "service_unavailable".to_owned(),
                message: message.into(),
                retryable: true,
                details: Vec::new(),
            },
        }
    }

    fn websocket_subprotocol() -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            body: ApiErrorBody {
                code: "websocket_subprotocol_required".to_owned(),
                message: format!("WebSocket subprotocol {EVENT_SUBPROTOCOL} is required."),
                retryable: false,
                details: Vec::new(),
            },
        }
    }

    fn task(error: impl std::fmt::Display) -> Self {
        Self::internal(error.to_string())
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            body: ApiErrorBody {
                code: "internal_error".to_owned(),
                message: message.into(),
                retryable: false,
                details: Vec::new(),
            },
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let unauthorized = self.status == StatusCode::UNAUTHORIZED;
        let mut response = (self.status, Json(self.body)).into_response();
        if unauthorized {
            response.headers_mut().insert(
                WWW_AUTHENTICATE,
                HeaderValue::from_static("Bearer realm=\"openbrief\""),
            );
        }
        response
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RemoteError {
    #[error("remote.token_file is required when remote access is enabled")]
    TokenPathRequired,
    #[error("could not read remote token file {path}: {source}")]
    TokenRead {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("remote token file must only be readable by its owner: {0}")]
    TokenPermissions(PathBuf),
    #[error("remote token must contain between 32 and 256 bytes")]
    TokenLength,
    #[error("could not bind remote API at {bind}: {message}")]
    Bind { bind: SocketAddr, message: String },
    #[error("remote API did not start within five seconds at {0}")]
    StartupTimeout(SocketAddr),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn token_file_must_be_private_and_long_enough() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("token");
        let mut file = fs::File::create(&path).unwrap();
        writeln!(file, "0123456789abcdef0123456789abcdef").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(
            load_token(&path).unwrap(),
            "0123456789abcdef0123456789abcdef"
        );
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(matches!(
            load_token(&path),
            Err(RemoteError::TokenPermissions(_))
        ));
    }
}
