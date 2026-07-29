#![allow(clippy::missing_errors_doc)]

mod config;
mod control;
mod paths;
mod presence;
mod query;
mod systemd;
mod watch;

pub use config::{CaptureConfig, Config, ConfigError};
pub use control::{
    CollectorStatus, ControlClient, ControlError, ControlRequest, ControlResponse, RecordingStatus,
};
pub use paths::{AppPaths, PathsError};
pub use query::{ContextDetail, ContextService, QueryError};
pub use systemd::{ServiceError, ServiceManager};
pub use watch::{WatchError, run_watch};
