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
    AppPaths, CollectorStatus, Config, ConfigError, ControlRequest, ControlResponse, PathsError,
    RecordingStatus,
    presence::{Presence, current_presence},
};

const CONTROL_MESSAGE_LIMIT: u64 = 64 * 1024;

pub fn run_watch() -> Result<(), WatchError> {
    let paths = AppPaths::discover()?;
    let config = Config::load_or_create(&paths.config_file)?;
    prepare_runtime(&paths)?;
    let mut store = Store::open(&paths.database_file)?;
    store.purge_before(now() - retention_duration(&config))?;

    let (sender, receiver) = mpsc::channel();
    spawn_niri(sender.clone());
    spawn_presence(sender.clone());
    spawn_control(&paths, sender.clone())?;
    let signal_sender = sender;
    ctrlc::set_handler(move || {
        let _ = signal_sender.send(WatchMessage::Shutdown);
    })
    .map_err(WatchError::Signal)?;

    let result = watch_loop(&config, &mut store, &receiver);
    let _ = fs::remove_file(&paths.control_socket);
    result
}

#[allow(clippy::too_many_lines)]
fn watch_loop(
    config: &Config,
    store: &mut Store,
    receiver: &Receiver<WatchMessage>,
) -> Result<(), WatchError> {
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
            WatchMessage::Focus(change) => {
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
            WatchMessage::SourceUnavailable => {
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
            WatchMessage::Presence(next) => {
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
            WatchMessage::Control(request, response) => {
                let reply = match request {
                    ControlRequest::Status => ControlResponse::Status(CollectorStatus {
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
                };
                let _ = response.send(reply);
            }
            WatchMessage::Shutdown => {
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
) -> Result<(), WatchError> {
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

fn spawn_niri(sender: Sender<WatchMessage>) {
    thread::spawn(move || {
        loop {
            match NiriEventSource::connect() {
                Ok(mut source) => loop {
                    match source.next_focus_change() {
                        Ok(Some(change)) => {
                            if sender.send(WatchMessage::Focus(change)).is_err() {
                                return;
                            }
                        }
                        Ok(None) => {}
                        Err(_) => {
                            let _ = sender.send(WatchMessage::SourceUnavailable);
                            break;
                        }
                    }
                },
                Err(_) => {
                    if sender.send(WatchMessage::SourceUnavailable).is_err() {
                        return;
                    }
                }
            }
            thread::sleep(StdDuration::from_secs(3));
        }
    });
}

fn spawn_presence(sender: Sender<WatchMessage>) {
    thread::spawn(move || {
        let mut previous = None;
        loop {
            let presence = current_presence();
            if previous != Some(presence) {
                if sender.send(WatchMessage::Presence(presence)).is_err() {
                    return;
                }
                previous = Some(presence);
            }
            thread::sleep(StdDuration::from_secs(2));
        }
    });
}

fn spawn_control(paths: &AppPaths, sender: Sender<WatchMessage>) -> Result<(), WatchError> {
    if paths.control_socket.exists() {
        fs::remove_file(&paths.control_socket)?;
    }
    let listener = UnixListener::bind(&paths.control_socket)?;
    fs::set_permissions(&paths.control_socket, fs::Permissions::from_mode(0o600))?;
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else {
                continue;
            };
            handle_control_stream(stream, &sender);
        }
    });
    Ok(())
}

fn handle_control_stream(mut stream: UnixStream, sender: &Sender<WatchMessage>) {
    let request = {
        let mut reader = BufReader::new(&mut stream).take(CONTROL_MESSAGE_LIMIT);
        let mut bytes = Vec::new();
        if reader.read_until(b'\n', &mut bytes).is_err() {
            return;
        }
        serde_json::from_slice::<ControlRequest>(&bytes)
    };
    let response = match request {
        Ok(request) => {
            let (reply_sender, reply_receiver) = mpsc::channel();
            if sender
                .send(WatchMessage::Control(request, reply_sender))
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
        Err(error) => ControlResponse::Error {
            code: "invalid_request".into(),
            message: error.to_string(),
        },
    };
    let _ = serde_json::to_writer(&mut stream, &response);
    let _ = stream.write_all(b"\n");
}

fn prepare_runtime(paths: &AppPaths) -> Result<(), WatchError> {
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

enum WatchMessage {
    Focus(NiriFocusChange),
    SourceUnavailable,
    Presence(Presence),
    Control(ControlRequest, Sender<ControlResponse>),
    Shutdown,
}

#[derive(Debug, thiserror::Error)]
pub enum WatchError {
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
    #[error("could not install signal handler: {0}")]
    Signal(ctrlc::Error),
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
