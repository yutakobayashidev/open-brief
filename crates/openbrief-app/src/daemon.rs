use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration as StdDuration;

use openbrief_core::{FocusState, FocusTransition};
use openbrief_source_niri::{NiriEventSource, NiriFocusChange};
use openbrief_store::Store;
use time::{Duration, OffsetDateTime};

use crate::{
    AgentHost, AgentHostRequest, AgentHostResponse, AppPaths, CollectorStatus, Config, ConfigError,
    ControlRequest, ControlResponse, EventJournal, PathsError, RecordingStatus, RemoteServer,
    agent_host::{apply_proposal, load_brief, load_return_thread},
    presence::{Presence, current_presence},
};

const CONTROL_MESSAGE_LIMIT: u64 = 64 * 1024;

pub fn run_daemon() -> Result<(), DaemonError> {
    let paths = AppPaths::discover()?;
    let config = Config::load_or_create(&paths.config_file)?;
    prepare_runtime(&paths)?;
    let mut store = Store::open(&paths.database_file)?;
    store.purge_before(now() - retention_duration(&config))?;

    let (sender, receiver) = mpsc::channel();
    spawn_niri(sender.clone());
    spawn_presence(sender.clone());
    let journal = EventJournal::default();
    let agent = AgentHost::spawn(paths.clone(), journal.clone());
    spawn_control(&paths, sender.clone(), agent.clone(), journal.clone())?;
    let mut remote = RemoteServer::start(
        &config.remote,
        paths.database_file.clone(),
        agent.clone(),
        journal,
    )?;
    let signal_sender = sender;
    ctrlc::set_handler(move || {
        let _ = signal_sender.send(DaemonMessage::Shutdown);
    })
    .map_err(DaemonError::Signal)?;

    let result = daemon_loop(&config, &mut store, &receiver);
    if let Some(remote) = remote.as_mut() {
        remote.stop();
    }
    let _ = agent.request(AgentHostRequest::Shutdown);
    let _ = fs::remove_file(&paths.control_socket);
    result
}

#[allow(clippy::too_many_lines)]
fn daemon_loop(
    config: &Config,
    store: &mut Store,
    receiver: &Receiver<DaemonMessage>,
) -> Result<(), DaemonError> {
    let mut recording = RecordingStatus::Active;
    let mut paused_until = None;
    let mut source_available = false;
    let mut last_window_event_at = None;
    let mut presence = Presence::Unavailable;
    let mut last_focus = None;
    let mut last_transition_at = now();
    let mut next_retention_at = last_transition_at + Duration::hours(1);

    loop {
        let message = match receiver.recv_timeout(StdDuration::from_secs(1)) {
            Ok(message) => Some(message),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
        };
        let at = strictly_after(now(), last_transition_at);

        if at >= next_retention_at {
            store.purge_before(at - retention_duration(config))?;
            next_retention_at = at + Duration::hours(1);
        }

        if recording == RecordingStatus::Paused
            && paused_until.is_some_and(|deadline| deadline <= at)
        {
            recording = RecordingStatus::Active;
            paused_until = None;
            let (state, app_id) = active_focus(config, source_available, last_focus.as_ref());
            append_state(store, at, state, app_id)?;
            last_transition_at = at;
        }

        let Some(message) = message else {
            continue;
        };
        match message {
            DaemonMessage::Focus(change) => {
                source_available = true;
                last_window_event_at = Some(at);
                last_focus = Some(change);
                if recording == RecordingStatus::Paused
                    || matches!(presence, Presence::Idle | Presence::Locked)
                {
                    continue;
                }
                let (state, app_id) = active_focus(config, source_available, last_focus.as_ref());
                append_state(store, at, state, app_id)?;
                last_transition_at = at;
            }
            DaemonMessage::SourceUnavailable => {
                let was_available = source_available;
                source_available = false;
                if was_available
                    && recording == RecordingStatus::Active
                    && presence == Presence::Active
                {
                    append_state(store, at, FocusState::SourceUnavailable, None)?;
                    last_transition_at = at;
                }
            }
            DaemonMessage::Presence(next) => {
                if next == presence {
                    continue;
                }
                let previous = presence;
                presence = next;
                if recording == RecordingStatus::Paused {
                    continue;
                }
                let (state, app_id) = match presence {
                    Presence::Idle => (FocusState::Idle, None),
                    Presence::Locked => (FocusState::Locked, None),
                    Presence::Active if matches!(previous, Presence::Idle | Presence::Locked) => {
                        active_focus(config, source_available, last_focus.as_ref())
                    }
                    Presence::Unavailable
                        if matches!(previous, Presence::Idle | Presence::Locked) =>
                    {
                        (FocusState::SourceUnavailable, None)
                    }
                    Presence::Active | Presence::Unavailable => continue,
                };
                append_state(store, at, state, app_id)?;
                last_transition_at = at;
            }
            DaemonMessage::Control(request, response) => {
                let reply = match request {
                    ControlRequest::Status => ControlResponse::Status(CollectorStatus {
                        control_protocol_version: openbrief_protocol::CONTROL_PROTOCOL_VERSION,
                        schema_version: 1,
                        recording,
                        last_window_event_at,
                        paused_until,
                        source_available,
                    }),
                    ControlRequest::Pause { until } => {
                        recording = RecordingStatus::Paused;
                        paused_until = until;
                        if let Err(error) = append_state(store, at, FocusState::Paused, None) {
                            ControlResponse::Error {
                                code: "store_error".into(),
                                message: error.to_string(),
                            }
                        } else {
                            last_transition_at = at;
                            ControlResponse::Ok
                        }
                    }
                    ControlRequest::Resume => {
                        if recording == RecordingStatus::Active {
                            let _ = response.send(ControlResponse::Ok);
                            continue;
                        }
                        recording = RecordingStatus::Active;
                        paused_until = None;
                        let (state, app_id) = match presence {
                            Presence::Idle => (FocusState::Idle, None),
                            Presence::Locked => (FocusState::Locked, None),
                            _ => active_focus(config, source_available, last_focus.as_ref()),
                        };
                        if let Err(error) = append_state(store, at, state, app_id) {
                            ControlResponse::Error {
                                code: "store_error".into(),
                                message: error.to_string(),
                            }
                        } else {
                            last_transition_at = at;
                            ControlResponse::Ok
                        }
                    }
                    ControlRequest::Delete { start, end } => match store.delete_range(start, end) {
                        Ok(deleted) => {
                            let next_at = strictly_after(now(), at);
                            let _ =
                                append_state(store, next_at, FocusState::SourceUnavailable, None);
                            last_transition_at = next_at;
                            ControlResponse::Deleted {
                                segments: u64::try_from(deleted).unwrap_or(u64::MAX),
                            }
                        }
                        Err(error) => ControlResponse::Error {
                            code: "store_error".into(),
                            message: error.to_string(),
                        },
                    },
                    ControlRequest::Shutdown => {
                        let _ = store.close_current_segment(at);
                        let _ = response.send(ControlResponse::Ok);
                        return Ok(());
                    }
                    _ => ControlResponse::Error {
                        code: "invalid_collector_request".into(),
                        message: "request was routed to the collector incorrectly".into(),
                    },
                };
                let _ = response.send(reply);
            }
            DaemonMessage::Shutdown => {
                let _ = store.close_current_segment(at);
                return Ok(());
            }
        }
    }
}

fn append_state(
    store: &mut Store,
    at: OffsetDateTime,
    state: FocusState,
    app_id: Option<String>,
) -> Result<(), DaemonError> {
    let transition = FocusTransition::new(at, state, app_id)?;
    store.append_transition(&transition)?;
    Ok(())
}

fn active_focus(
    config: &Config,
    source_available: bool,
    change: Option<&NiriFocusChange>,
) -> (FocusState, Option<String>) {
    if !source_available {
        return (FocusState::SourceUnavailable, None);
    }
    let Some(app_id) = change.and_then(|change| change.app_id.as_deref()) else {
        return (FocusState::SourceUnavailable, None);
    };
    if config.is_excluded(app_id) {
        (FocusState::Excluded, None)
    } else {
        (FocusState::Observed, Some(app_id.to_owned()))
    }
}

fn retention_duration(config: &Config) -> Duration {
    Duration::days(i64::from(config.retention_days))
}

fn spawn_niri(sender: Sender<DaemonMessage>) {
    thread::spawn(move || {
        loop {
            match NiriEventSource::connect() {
                Ok(mut source) => loop {
                    match source.next_focus_change() {
                        Ok(Some(change)) => {
                            if sender.send(DaemonMessage::Focus(change)).is_err() {
                                return;
                            }
                        }
                        Ok(None) => {}
                        Err(_) => {
                            let _ = sender.send(DaemonMessage::SourceUnavailable);
                            break;
                        }
                    }
                },
                Err(_) => {
                    if sender.send(DaemonMessage::SourceUnavailable).is_err() {
                        return;
                    }
                }
            }
            thread::sleep(StdDuration::from_secs(3));
        }
    });
}

fn spawn_presence(sender: Sender<DaemonMessage>) {
    thread::spawn(move || {
        let mut previous = None;
        loop {
            let presence = current_presence();
            if previous != Some(presence) {
                if sender.send(DaemonMessage::Presence(presence)).is_err() {
                    return;
                }
                previous = Some(presence);
            }
            thread::sleep(StdDuration::from_secs(2));
        }
    });
}

fn spawn_control(
    paths: &AppPaths,
    sender: Sender<DaemonMessage>,
    agent: AgentHost,
    journal: EventJournal,
) -> Result<(), DaemonError> {
    if paths.control_socket.exists() {
        if UnixStream::connect(&paths.control_socket).is_ok() {
            return Err(DaemonError::AlreadyRunning(paths.control_socket.clone()));
        }
        fs::remove_file(&paths.control_socket)?;
    }
    let listener = UnixListener::bind(&paths.control_socket)?;
    fs::set_permissions(&paths.control_socket, fs::Permissions::from_mode(0o600))?;
    let database_file = paths.database_file.clone();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else {
                continue;
            };
            let sender = sender.clone();
            let agent = agent.clone();
            let journal = journal.clone();
            let database_file = database_file.clone();
            thread::spawn(move || {
                handle_control_stream(stream, &sender, &agent, &journal, &database_file);
            });
        }
    });
    Ok(())
}

fn handle_control_stream(
    mut stream: UnixStream,
    sender: &Sender<DaemonMessage>,
    agent: &AgentHost,
    journal: &EventJournal,
    database_file: &std::path::Path,
) {
    let request = {
        let mut reader = BufReader::new(&mut stream).take(CONTROL_MESSAGE_LIMIT);
        let mut bytes = Vec::new();
        if reader.read_until(b'\n', &mut bytes).is_err() {
            return;
        }
        serde_json::from_slice::<ControlRequest>(&bytes)
    };
    let response = match request {
        Ok(request) if is_collector_request(&request) => {
            let (reply_sender, reply_receiver) = mpsc::channel();
            if sender
                .send(DaemonMessage::Control(request, reply_sender))
                .is_err()
            {
                ControlResponse::Error {
                    code: "collector_stopped".into(),
                    message: "collector is shutting down".into(),
                }
            } else {
                reply_receiver
                    .recv_timeout(StdDuration::from_secs(2))
                    .unwrap_or_else(|_| ControlResponse::Error {
                        code: "collector_timeout".into(),
                        message: "collector did not respond in time".into(),
                    })
            }
        }
        Ok(request) => handle_application_request(request, agent, journal, database_file),
        Err(error) => ControlResponse::Error {
            code: "invalid_request".into(),
            message: error.to_string(),
        },
    };
    let _ = serde_json::to_writer(&mut stream, &response);
    let _ = stream.write_all(b"\n");
}

fn is_collector_request(request: &ControlRequest) -> bool {
    matches!(
        request,
        ControlRequest::Status
            | ControlRequest::Pause { .. }
            | ControlRequest::Resume
            | ControlRequest::Delete { .. }
            | ControlRequest::Shutdown
    )
}

fn handle_application_request(
    request: ControlRequest,
    agent: &AgentHost,
    journal: &EventJournal,
    database_file: &std::path::Path,
) -> ControlResponse {
    let result = match request {
        ControlRequest::LoadBrief => load_brief(database_file).map(ControlResponse::Brief),
        ControlRequest::LoadReturnThread => {
            load_return_thread(database_file).map(ControlResponse::ReturnThread)
        }
        ControlRequest::ApplyProposal { proposal_id } => {
            apply_proposal(database_file, &proposal_id).map(|proposal| {
                journal.push(crate::DaemonEvent::ProposalApplied {
                    proposal: proposal.clone(),
                });
                ControlResponse::Proposal(proposal)
            })
        }
        ControlRequest::AgentStatus => map_agent_response(agent.request(AgentHostRequest::Status)),
        ControlRequest::AgentStart => map_agent_response(agent.request(AgentHostRequest::Start)),
        ControlRequest::AgentAuthenticate { method_id } => {
            map_agent_response(agent.request(AgentHostRequest::Authenticate { method_id }))
        }
        ControlRequest::AgentPrompt { text } => {
            map_agent_response(agent.request(AgentHostRequest::Prompt { text }))
        }
        ControlRequest::AgentCancel => map_agent_response(agent.request(AgentHostRequest::Cancel)),
        ControlRequest::AgentStop => map_agent_response(agent.request(AgentHostRequest::Stop)),
        ControlRequest::EventCursor => Ok(ControlResponse::EventCursor {
            next_sequence: journal.cursor(),
        }),
        ControlRequest::Events { after } => {
            let (events, next_sequence) = journal.after(after);
            Ok(ControlResponse::Events {
                events,
                next_sequence,
            })
        }
        ControlRequest::Status
        | ControlRequest::Pause { .. }
        | ControlRequest::Resume
        | ControlRequest::Delete { .. }
        | ControlRequest::Shutdown => Err("collector request was routed incorrectly".to_owned()),
    };
    result.unwrap_or_else(|message| ControlResponse::Error {
        code: "application_error".into(),
        message,
    })
}

fn map_agent_response(
    response: Result<AgentHostResponse, crate::agent_host::AgentHostError>,
) -> Result<ControlResponse, String> {
    response
        .map(|response| match response {
            AgentHostResponse::Status(status) => ControlResponse::AgentStatus(status),
            AgentHostResponse::TurnStarted { .. } | AgentHostResponse::Ok => ControlResponse::Ok,
        })
        .map_err(|error| error.to_string())
}

fn prepare_runtime(paths: &AppPaths) -> Result<(), DaemonError> {
    fs::create_dir_all(&paths.runtime_dir)?;
    fs::set_permissions(&paths.runtime_dir, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

fn now() -> OffsetDateTime {
    OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())
}

fn strictly_after(value: OffsetDateTime, previous: OffsetDateTime) -> OffsetDateTime {
    if value <= previous {
        previous + Duration::nanoseconds(1)
    } else {
        value
    }
}

enum DaemonMessage {
    Focus(NiriFocusChange),
    SourceUnavailable,
    Presence(Presence),
    Control(ControlRequest, Sender<ControlResponse>),
    Shutdown,
}

#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    #[error(transparent)]
    Paths(#[from] PathsError),
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Store(#[from] openbrief_store::StoreError),
    #[error(transparent)]
    Core(#[from] openbrief_core::CoreError),
    #[error("runtime I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("OpenBrief daemon is already running at {0}")]
    AlreadyRunning(std::path::PathBuf),
    #[error("could not install signal handler: {0}")]
    Signal(ctrlc::Error),
    #[error(transparent)]
    Remote(#[from] crate::remote::RemoteError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excluded_app_never_keeps_its_identity() {
        let config = Config::default();
        let change = NiriFocusChange {
            app_id: Some("org.signal.Signal".into()),
            window_key: Some("opaque".into()),
        };
        let (state, app_id) = active_focus(&config, true, Some(&change));
        assert_eq!(state, FocusState::Excluded);
        assert_eq!(app_id, None);
    }

    #[test]
    fn active_focus_restores_the_cached_application() {
        let config = Config::default();
        let change = NiriFocusChange {
            app_id: Some("com.mitchellh.ghostty".into()),
            window_key: Some("opaque".into()),
        };
        assert_eq!(
            active_focus(&config, true, Some(&change)),
            (FocusState::Observed, Some("com.mitchellh.ghostty".into()))
        );
        assert_eq!(
            active_focus(&config, false, Some(&change)),
            (FocusState::SourceUnavailable, None)
        );
    }

    #[test]
    fn retention_uses_the_configured_window() {
        let config = Config {
            retention_days: 3,
            ..Config::default()
        };
        assert_eq!(retention_duration(&config), Duration::days(3));
    }
}
