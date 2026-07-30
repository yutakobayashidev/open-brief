import type {
  AgentStatus,
  Brief,
  ConversationMessage,
  DesktopEvent,
  TriageProposal,
} from "./domain";

export type AppState = {
  agentStatus: AgentStatus;
  brief: Brief | null;
  messages: ConversationMessage[];
  proposal: TriageProposal | null;
  appliedProposal: TriageProposal | null;
  isSending: boolean;
  error: string | null;
};

export type AppAction =
  | { type: "brief_loaded"; brief: Brief }
  | { type: "user_message_sent"; message: ConversationMessage }
  | { type: "proposal_rejected" }
  | { type: "error_dismissed" }
  | DesktopEvent;

export const initialState: AppState = {
  agentStatus: {
    availability: { status: "checking" },
    authentication: { status: "unknown" },
    process: { status: "stopped" },
    runtime: null,
  },
  brief: null,
  messages: [
    {
      id: "welcome",
      role: "agent",
      text: "判断を一文で教えてください。例：木曜のメールは今日、memoryの記事は8分だけ見る。",
      state: "complete",
    },
  ],
  proposal: null,
  appliedProposal: null,
  isSending: false,
  error: null,
};

export function appReducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case "brief_loaded":
      return { ...state, brief: action.brief };
    case "brief_changed":
      return { ...state, brief: action.brief };
    case "agent_status_changed":
      return { ...state, agentStatus: action.status };
    case "user_message_sent":
      return {
        ...state,
        messages: [...state.messages, action.message],
        proposal: null,
        isSending: true,
        error: null,
      };
    case "message_started":
      return {
        ...state,
        messages: [
          ...state.messages,
          { id: action.id, role: action.role, text: "", state: "streaming" },
        ],
      };
    case "message_delta":
      return {
        ...state,
        messages: state.messages.map((message) =>
          message.id === action.id
            ? { ...message, text: message.text + action.text }
            : message,
        ),
      };
    case "message_finished":
      return {
        ...state,
        messages: state.messages.map((message) =>
          message.id === action.id
            ? { ...message, state: "complete" }
            : message,
        ),
      };
    case "proposal_received":
      return { ...state, proposal: action.proposal };
    case "proposal_applied":
      return {
        ...state,
        proposal: null,
        appliedProposal: action.proposal,
      };
    case "proposal_rejected":
      return { ...state, proposal: null };
    case "turn_finished":
      return { ...state, isSending: false };
    case "error":
      return { ...state, error: action.message, isSending: false };
    case "error_dismissed":
      return { ...state, error: null };
  }
}
