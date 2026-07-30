use std::fs::{self, OpenOptions};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use openbrief_client::{LocalControlClient, LocalControlError};
use openbrief_protocol::{
    AgentStatus, BriefView, CONTROL_PROTOCOL_VERSION, ControlRequest, ControlResponse, DaemonEvent,
    TriageProposalView,
};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State};

const DAEMON_START_TIMEOUT: Duration = Duration::from_secs(5);
const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(100);

struct DesktopState {
    client: LocalControlClient,
    owned_daemon: Mutex<Option<Child>>,
    stop_forwarder: Arc<AtomicBool>,
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn load_brief(state: State<'_, DesktopState>) -> Result<BriefView, String> {
    match request(&state.client, &ControlRequest::LoadBrief)? {
        ControlResponse::Brief(brief) => Ok(brief),
        response => Err(unexpected(&response)),
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn load_return_thread(
    state: State<'_, DesktopState>,
) -> Result<Option<TriageProposalView>, String> {
    match request(&state.client, &ControlRequest::LoadReturnThread)? {
        ControlResponse::ReturnThread(proposal) => Ok(proposal),
        response => Err(unexpected(&response)),
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn agent_status(state: State<'_, DesktopState>) -> Result<AgentStatus, String> {
    request_agent_status(&state.client, &ControlRequest::AgentStatus)
}

#[tauri::command]
async fn agent_start(state: State<'_, DesktopState>) -> Result<AgentStatus, String> {
    let client = state.client.clone();
    run_blocking(move || request_agent_status(&client, &ControlRequest::AgentStart)).await
}

#[tauri::command]
async fn agent_authenticate(
    method_id: String,
    state: State<'_, DesktopState>,
) -> Result<AgentStatus, String> {
    let client = state.client.clone();
    run_blocking(move || {
        request_agent_status(&client, &ControlRequest::AgentAuthenticate { method_id })
    })
    .await
}

#[tauri::command]
async fn agent_prompt(text: String, state: State<'_, DesktopState>) -> Result<(), String> {
    let client = state.client.clone();
    run_blocking(move || expect_ok(request(&client, &ControlRequest::AgentPrompt { text })?)).await
}

#[tauri::command]
async fn agent_cancel(state: State<'_, DesktopState>) -> Result<(), String> {
    let client = state.client.clone();
    run_blocking(move || expect_ok(request(&client, &ControlRequest::AgentCancel)?)).await
}

#[tauri::command]
async fn agent_stop(state: State<'_, DesktopState>) -> Result<(), String> {
    let client = state.client.clone();
    run_blocking(move || expect_ok(request(&client, &ControlRequest::AgentStop)?)).await
}

#[tauri::command]
async fn apply_proposal(proposal_id: String, state: State<'_, DesktopState>) -> Result<(), String> {
    let client = state.client.clone();
    run_blocking(
        move || match request(&client, &ControlRequest::ApplyProposal { proposal_id })? {
            ControlResponse::Proposal(_) => Ok(()),
            response => Err(unexpected(&response)),
        },
    )
    .await
}

async fn run_blocking<T: Send + 'static>(
    operation: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(|error| format!("daemon request task failed: {error}"))?
}

fn request_agent_status(
    client: &LocalControlClient,
    command: &ControlRequest,
) -> Result<AgentStatus, String> {
    match request(client, command)? {
        ControlResponse::AgentStatus(status) => Ok(status),
        response => Err(unexpected(&response)),
    }
}

fn expect_ok(response: ControlResponse) -> Result<(), String> {
    match response {
        ControlResponse::Ok => Ok(()),
        response => Err(unexpected(&response)),
    }
}

fn request(
    client: &LocalControlClient,
    command: &ControlRequest,
) -> Result<ControlResponse, String> {
    match client.request(command).map_err(|error| error.to_string())? {
        ControlResponse::Error { code, message } => Err(format!("{message} ({code})")),
        response => Ok(response),
    }
}

fn unexpected(response: &ControlResponse) -> String {
    format!("openbriefd returned an unexpected response: {response:?}")
}

fn setup_desktop(app: &mut tauri::App) -> Result<(), String> {
    let paths = openbrief_app::AppPaths::discover().map_err(|error| error.to_string())?;
    let client = LocalControlClient::new(&paths.control_socket);
    let owned_daemon = ensure_daemon(&client, &paths)?;
    let cursor = match request(&client, &ControlRequest::EventCursor)? {
        ControlResponse::EventCursor { next_sequence } => next_sequence,
        response => return Err(unexpected(&response)),
    };
    let stop_forwarder = Arc::new(AtomicBool::new(false));
    start_event_forwarder(
        app.handle().clone(),
        client.clone(),
        stop_forwarder.clone(),
        cursor,
    );
    app.manage(DesktopState {
        client,
        owned_daemon: Mutex::new(owned_daemon),
        stop_forwarder,
    });
    Ok(())
}

fn ensure_daemon(
    client: &LocalControlClient,
    paths: &openbrief_app::AppPaths,
) -> Result<Option<Child>, String> {
    match client.request(&ControlRequest::Status) {
        Ok(ControlResponse::Status(status)) => {
            ensure_compatible_daemon(status.control_protocol_version)?;
            return Ok(None);
        }
        Err(LocalControlError::Connect { .. }) => {}
        Ok(response) => return Err(unexpected(&response)),
        Err(error) => return Err(format!("existing openbriefd is unhealthy: {error}")),
    }

    fs::create_dir_all(&paths.runtime_dir).map_err(display_error)?;
    let executable = sibling_executable("openbriefd")?;
    let log_path = paths.runtime_dir.join("openbriefd.log");
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(display_error)?;
    let stderr = stdout.try_clone().map_err(display_error)?;
    let mut command = Command::new(&executable);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("could not start {}: {error}", executable.display()))?;
    let deadline = Instant::now() + DAEMON_START_TIMEOUT;
    loop {
        match client.request(&ControlRequest::Status) {
            Ok(ControlResponse::Status(status)) => {
                if let Err(error) = ensure_compatible_daemon(status.control_protocol_version) {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(error);
                }
                return Ok(Some(child));
            }
            Ok(response) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(unexpected(&response));
            }
            Err(_) if Instant::now() < deadline => {
                if let Some(status) = child.try_wait().map_err(display_error)? {
                    return Err(format!(
                        "openbriefd exited during startup with {status}; see {}",
                        log_path.display()
                    ));
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "openbriefd did not become ready: {error}; see {}",
                    log_path.display()
                ));
            }
        }
    }
}

fn ensure_compatible_daemon(version: u32) -> Result<(), String> {
    if version == CONTROL_PROTOCOL_VERSION {
        Ok(())
    } else {
        Err(format!(
            "openbriefd control protocol {version} is incompatible with desktop protocol \
             {CONTROL_PROTOCOL_VERSION}; restart or upgrade the openbriefd service"
        ))
    }
}

fn start_event_forwarder(
    app: AppHandle,
    client: LocalControlClient,
    stop: Arc<AtomicBool>,
    mut cursor: u64,
) {
    thread::spawn(move || {
        let mut reported_error = false;
        while !stop.load(Ordering::Acquire) {
            match client.request(&ControlRequest::Events { after: cursor }) {
                Ok(ControlResponse::Events {
                    events,
                    next_sequence,
                }) => {
                    cursor = next_sequence;
                    reported_error = false;
                    for event in events {
                        let _ = app.emit("desktop-event", event.event);
                    }
                }
                Ok(response) => {
                    if !reported_error {
                        emit_error(&app, unexpected(&response));
                        reported_error = true;
                    }
                }
                Err(error) => {
                    if !reported_error {
                        emit_error(&app, error.to_string());
                        reported_error = true;
                    }
                }
            }
            thread::sleep(EVENT_POLL_INTERVAL);
        }
    });
}

fn emit_error(app: &AppHandle, message: String) {
    let _ = app.emit("desktop-event", DaemonEvent::Error { message });
}

fn sibling_executable(name: &str) -> Result<std::path::PathBuf, String> {
    let current = std::env::current_exe().map_err(display_error)?;
    let parent = current
        .parent()
        .ok_or_else(|| "OpenBrief executable has no parent directory".to_owned())?;
    let executable = parent.join(name);
    if executable.is_file() {
        Ok(executable)
    } else {
        Err(format!(
            "required OpenBrief executable is missing: {}",
            executable.display()
        ))
    }
}

fn shutdown_owned_daemon(app: &AppHandle) {
    let state = app.state::<DesktopState>();
    state.stop_forwarder.store(true, Ordering::Release);
    let Some(mut child) = state
        .owned_daemon
        .lock()
        .expect("owned daemon lock poisoned")
        .take()
    else {
        return;
    };
    let _ = state.client.request(&ControlRequest::Shutdown);
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(50)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
        }
    }
}

fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

/// Starts the desktop application.
///
/// # Panics
///
/// Panics when the Tauri runtime cannot be initialized or exits with an error.
pub fn run() {
    let app = tauri::Builder::default()
        .setup(|app| {
            setup_desktop(app).map_err(|error| {
                Box::new(std::io::Error::other(error)) as Box<dyn std::error::Error>
            })
        })
        .invoke_handler(tauri::generate_handler![
            load_brief,
            load_return_thread,
            agent_status,
            agent_start,
            agent_authenticate,
            agent_prompt,
            agent_cancel,
            agent_stop,
            apply_proposal,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build OpenBrief desktop");
    app.run(|app, event| {
        if matches!(event, RunEvent::Exit) {
            shutdown_owned_daemon(app);
        }
    });
}
