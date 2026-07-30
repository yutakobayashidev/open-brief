use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

pub const CONTROL_PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeDescriptor {
    pub provider_id: String,
    pub label: String,
    pub source: RuntimeSource,
    pub version: Option<String>,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeSource {
    Packaged,
    NixStore,
    Override,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthMethodView {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BriefView {
    pub protect: Vec<BriefItemView>,
    pub explore: Vec<ExplorationView>,
    pub coverage: Vec<CoverageView>,
    pub generated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BriefItemView {
    pub id: String,
    pub title: String,
    pub reason: String,
    pub source: String,
    pub observed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplorationView {
    #[serde(flatten)]
    pub item: BriefItemView,
    pub minutes: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageView {
    pub source: String,
    pub observed_at: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriageProposalView {
    pub id: String,
    pub summary: String,
    pub protect_ids: Vec<String>,
    pub explore_id: Option<String>,
    pub return_anchor: String,
    pub return_command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatus {
    pub availability: AvailabilityStatus,
    pub authentication: AuthenticationStatus,
    pub process: ProcessStatus,
    pub runtime: Option<RuntimeDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AvailabilityStatus {
    Checking,
    Available,
    Missing { message: String },
    Incompatible { found: String, required: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AuthenticationStatus {
    Unknown,
    Authenticating,
    Required { methods: Vec<AuthMethodView> },
    Authenticated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ProcessStatus {
    Stopped,
    Starting,
    Ready,
    Busy,
    Failed { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonEvent {
    AgentStatusChanged { status: AgentStatus },
    MessageStarted { id: String, role: String },
    MessageDelta { id: String, text: String },
    MessageFinished { id: String },
    ProposalReceived { proposal: TriageProposalView },
    BriefChanged { brief: BriefView },
    ProposalApplied { proposal: TriageProposalView },
    TurnFinished,
    Error { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SequencedDaemonEvent {
    pub sequence: u64,
    pub event: DaemonEvent,
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
    LoadBrief,
    LoadReturnThread,
    ApplyProposal {
        proposal_id: String,
    },
    AgentStatus,
    AgentStart,
    AgentAuthenticate {
        method_id: String,
    },
    AgentPrompt {
        text: String,
    },
    AgentCancel,
    AgentStop,
    EventCursor,
    Events {
        after: u64,
    },
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", content = "data", rename_all = "snake_case")]
pub enum ControlResponse {
    Ok,
    Status(CollectorStatus),
    Deleted {
        segments: u64,
    },
    Brief(BriefView),
    ReturnThread(Option<TriageProposalView>),
    Proposal(TriageProposalView),
    AgentStatus(AgentStatus),
    EventCursor {
        next_sequence: u64,
    },
    Events {
        events: Vec<SequencedDaemonEvent>,
        next_sequence: u64,
    },
    Error {
        code: String,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectorStatus {
    #[serde(default)]
    pub control_protocol_version: u32,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub data: T,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSnapshot {
    pub brief: BriefView,
    pub return_thread: Option<TriageProposalView>,
    pub pending_proposal: Option<TriageProposalView>,
    pub agent: AgentStatus,
    pub next_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartTurnRequest {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartedTurn {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub details: Vec<ApiErrorDetail>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiErrorDetail {
    pub field: String,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_wire_format_is_stable() {
        let event = DaemonEvent::TurnFinished;
        assert_eq!(
            serde_json::to_value(event).unwrap(),
            serde_json::json!({"type": "turn_finished"})
        );
    }

    #[test]
    fn control_wire_format_is_stable() {
        let request = ControlRequest::AgentPrompt {
            text: "整理して".to_owned(),
        };
        assert_eq!(
            serde_json::to_value(request).unwrap(),
            serde_json::json!({
                "method": "agent_prompt",
                "params": {"text": "整理して"}
            })
        );
    }
}
