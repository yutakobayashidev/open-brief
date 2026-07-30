#![allow(clippy::missing_errors_doc)]

mod attention;
mod config;
mod control;
mod mcp;
mod paths;
mod presence;
mod query;
mod systemd;
mod watch;

pub use attention::{
    AttentionError, AttentionService, ConfirmedTriage, IngestOutcome, StoredTriageProposal,
};
pub use config::{AgentSettings, CaptureConfig, Config, ConfigError};
pub use control::{
    CollectorStatus, ControlClient, ControlError, ControlRequest, ControlResponse, RecordingStatus,
};
pub use mcp::{McpServerError, ProposalMcpServer, run_proposal_mcp_server};
pub use paths::{AppPaths, PathsError};
pub use query::{ContextDetail, ContextService, QueryError};
pub use systemd::{ServiceError, ServiceManager};
pub use watch::{WatchError, run_watch};
