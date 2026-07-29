use std::io::{BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

const MAX_CONTROL_MESSAGE_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone)]
pub struct ControlClient {
    socket: PathBuf,
}

impl ControlClient {
    pub fn new(socket: impl Into<PathBuf>) -> Self {
        Self {
            socket: socket.into(),
        }
    }

    pub fn request(&self, request: &ControlRequest) -> Result<ControlResponse, ControlError> {
        let mut stream =
            UnixStream::connect(&self.socket).map_err(|source| ControlError::Connect {
                path: self.socket.clone(),
                source,
            })?;
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .map_err(ControlError::Io)?;
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .map_err(ControlError::Io)?;

        serde_json::to_writer(&mut stream, request).map_err(ControlError::Encode)?;
        stream.write_all(b"\n").map_err(ControlError::Io)?;
        stream.flush().map_err(ControlError::Io)?;

        let reader = BufReader::new(stream).take(MAX_CONTROL_MESSAGE_BYTES);
        serde_json::from_reader(reader).map_err(ControlError::Decode)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "method", content = "params", rename_all = "snake_case")]
pub enum ControlRequest {
    Status,
    Pause {
        #[serde(with = "time::serde::rfc3339::option")]
        until: Option<OffsetDateTime>,
    },
    Resume,
    Delete {
        #[serde(with = "time::serde::rfc3339")]
        start: OffsetDateTime,
        #[serde(with = "time::serde::rfc3339")]
        end: OffsetDateTime,
    },
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", content = "data", rename_all = "snake_case")]
pub enum ControlResponse {
    Ok,
    Status(CollectorStatus),
    Deleted { segments: u64 },
    Error { code: String, message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectorStatus {
    pub schema_version: u32,
    pub recording: RecordingStatus,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_window_event_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub paused_until: Option<OffsetDateTime>,
    pub source_available: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingStatus {
    Active,
    Paused,
    Disabled,
}

#[derive(Debug, thiserror::Error)]
pub enum ControlError {
    #[error("could not connect to control socket {path}: {source}")]
    Connect {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("control socket I/O failed: {0}")]
    Io(std::io::Error),
    #[error("could not encode control request: {0}")]
    Encode(serde_json::Error),
    #[error("could not decode control response: {0}")]
    Decode(serde_json::Error),
}
