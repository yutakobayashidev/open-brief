use std::io::{BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use openbrief_protocol::{ControlRequest, ControlResponse};

const MAX_CONTROL_MESSAGE_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone)]
pub struct LocalControlClient {
    socket: PathBuf,
}

impl LocalControlClient {
    #[must_use]
    pub fn new(socket: impl Into<PathBuf>) -> Self {
        Self {
            socket: socket.into(),
        }
    }

    /// Sends one request to a local `OpenBrief` daemon.
    ///
    /// # Errors
    ///
    /// Returns an error when the socket cannot be reached or the response is invalid.
    pub fn request(&self, request: &ControlRequest) -> Result<ControlResponse, LocalControlError> {
        let mut stream =
            UnixStream::connect(&self.socket).map_err(|source| LocalControlError::Connect {
                path: self.socket.clone(),
                source,
            })?;
        stream
            .set_read_timeout(Some(request_timeout(request)))
            .map_err(LocalControlError::Io)?;
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .map_err(LocalControlError::Io)?;

        serde_json::to_writer(&mut stream, request).map_err(LocalControlError::Encode)?;
        stream.write_all(b"\n").map_err(LocalControlError::Io)?;
        stream.flush().map_err(LocalControlError::Io)?;

        let reader = BufReader::new(stream).take(MAX_CONTROL_MESSAGE_BYTES);
        serde_json::from_reader(reader).map_err(LocalControlError::Decode)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LocalControlError {
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

fn request_timeout(request: &ControlRequest) -> Duration {
    match request {
        ControlRequest::AgentAuthenticate { .. } => Duration::from_secs(130),
        ControlRequest::AgentStart => Duration::from_secs(20),
        _ => Duration::from_secs(5),
    }
}
