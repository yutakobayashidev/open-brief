#![allow(clippy::missing_errors_doc)]

mod agent_host;
mod agent_runtime;
mod attention;
mod config;
mod daemon;
mod event_journal;
mod mcp;
mod paths;
mod presence;
mod query;
mod remote;
mod systemd;

pub(crate) use agent_host::{AgentHost, AgentHostRequest, AgentHostResponse};
pub use attention::{
    AttentionError, AttentionService, ConfirmedTriage, IngestOutcome, StoredTriageProposal,
};
pub use config::{AgentSettings, CaptureConfig, Config, ConfigError, RemoteSettings};
pub use daemon::{DaemonError, run_daemon};
pub(crate) use event_journal::EventJournal;
pub use mcp::{McpServerError, ProposalMcpServer, run_proposal_mcp_server};
pub(crate) use openbrief_client::{LocalControlClient, LocalControlError};
pub(crate) use openbrief_protocol::{
    AgentStatus, AuthMethodView, AuthenticationStatus, AvailabilityStatus, BriefItemView,
    BriefView, CollectorStatus, ControlRequest, ControlResponse, CoverageView, DaemonEvent,
    ExplorationView, ProcessStatus, RecordingStatus, SequencedDaemonEvent, TriageProposalView,
};
pub use paths::{AppPaths, PathsError};
pub use query::{ContextDetail, ContextService, QueryError};
pub(crate) use remote::RemoteServer;
pub use systemd::{ServiceError, ServiceManager};
