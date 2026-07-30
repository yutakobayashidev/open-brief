export type AvailabilityStatus =
  | { status: "checking" }
  | { status: "available" }
  | { status: "missing"; message: string }
  | { status: "incompatible"; found: string; required: string };

export type AuthMethod = {
  id: string;
  name: string;
};

export type AuthenticationStatus =
  | { status: "unknown" }
  | { status: "authenticating" }
  | { status: "required"; methods: AuthMethod[] }
  | { status: "authenticated" };

export type ProcessStatus =
  | { status: "stopped" }
  | { status: "starting" }
  | { status: "ready" }
  | { status: "busy" }
  | { status: "failed"; message: string };

export type RuntimeDescriptor = {
  providerId: string;
  label: string;
  source: "packaged" | "nix_store" | "override";
  version: string | null;
  path: string;
};

export type AgentStatus = {
  availability: AvailabilityStatus;
  authentication: AuthenticationStatus;
  process: ProcessStatus;
  runtime: RuntimeDescriptor | null;
};

export type Coverage = {
  source: string;
  observedAt: string;
  status: "fresh" | "stale";
};

export type BriefItem = {
  id: string;
  title: string;
  reason: string;
  source: string;
  observedAt: string;
};

export type Exploration = BriefItem & {
  minutes: number;
};

export type Brief = {
  protect: BriefItem[];
  explore: Exploration[];
  coverage: Coverage[];
  generatedAt: string;
};

export type TriageProposal = {
  id: string;
  summary: string;
  protectIds: string[];
  exploreId: string | null;
  returnAnchor: string;
  returnCommand: string;
};

export type ConversationMessage = {
  id: string;
  role: "agent" | "user";
  text: string;
  state?: "streaming" | "complete";
};

export type DesktopEvent =
  | { type: "agent_status_changed"; status: AgentStatus }
  | { type: "message_started"; id: string; role: "agent" }
  | { type: "message_delta"; id: string; text: string }
  | { type: "message_finished"; id: string }
  | { type: "proposal_received"; proposal: TriageProposal }
  | { type: "brief_changed"; brief: Brief }
  | { type: "proposal_applied"; proposal: TriageProposal }
  | { type: "turn_finished" }
  | { type: "error"; message: string };
