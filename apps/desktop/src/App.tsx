import { useEffect, useReducer } from "react";
import { BriefWorkspace } from "./components/brief-workspace";
import { Conversation } from "./components/conversation";
import { ReadinessBadge } from "./components/readiness";
import { ReturnThread } from "./components/return-thread";
import { Button } from "./components/ui";
import type { DesktopPort } from "./desktop-port";
import { appReducer, initialState } from "./state";

export function App({ port }: { port: DesktopPort }) {
  const [state, dispatch] = useReducer(appReducer, initialState);

  useEffect(() => {
    const disconnect = port.connect(dispatch);
    port
      .loadBrief()
      .then((brief) => dispatch({ type: "brief_loaded", brief }))
      .catch((error: unknown) =>
        dispatch({
          type: "error",
          message:
            error instanceof Error ? error.message : "Briefを読み込めませんでした",
        }),
      );
    return disconnect;
  }, [port]);

  const agentAvailable =
    state.agentStatus.availability.status === "available" &&
    state.agentStatus.authentication.status === "authenticated" &&
    ["ready", "busy"].includes(state.agentStatus.process.status);

  const updateAgentStatus = () => {
    port
      .retryReadiness()
      .then((status) => dispatch({ type: "agent_status_changed", status }))
      .catch((error: unknown) =>
        dispatch({
          type: "error",
          message: error instanceof Error ? error.message : String(error),
        }),
      );
  };

  const authenticate = (methodId: string) => {
    port
      .authenticate(methodId)
      .then((status) => dispatch({ type: "agent_status_changed", status }))
      .catch((error: unknown) =>
        dispatch({
          type: "error",
          message: error instanceof Error ? error.message : String(error),
        }),
      );
  };

  const send = (text: string) => {
    dispatch({
      type: "user_message_sent",
      message: {
        id: `user-${Date.now()}`,
        role: "user",
        text,
        state: "complete",
      },
    });
    port.sendMessage(text).catch((error: unknown) =>
      dispatch({
        type: "error",
        message:
          error instanceof Error
            ? error.message
            : `${state.agentStatus.runtime?.label ?? "Agent"}へ送信できませんでした`,
      }),
    );
  };

  const applyProposal = () => {
    if (!state.proposal) return;
    port.applyProposal(state.proposal.id).catch((error: unknown) =>
      dispatch({
        type: "error",
        message:
          error instanceof Error ? error.message : "提案を確定できませんでした",
      }),
    );
  };

  const rejectProposal = () => {
    if (!state.proposal) return;
    port
      .rejectProposal(state.proposal.id)
      .then(() => dispatch({ type: "proposal_rejected" }))
      .catch((error: unknown) =>
        dispatch({
          type: "error",
          message:
            error instanceof Error ? error.message : "提案を戻せませんでした",
        }),
      );
  };

  return (
    <div className="app-shell">
      <header className="topbar">
        <div className="brand">
          <span className="brand__mark" aria-hidden>
            ob
          </span>
          <strong>OpenBrief</strong>
          <span>attention handoff</span>
        </div>
        <ReadinessBadge
          status={state.agentStatus}
          onRetry={updateAgentStatus}
          onAuthenticate={authenticate}
        />
      </header>

      {state.error && (
        <div className="error-banner" role="alert">
          <span>{state.error}</span>
          <Button
            size="small"
            variant="danger"
            onClick={() => dispatch({ type: "error_dismissed" })}
          >
            閉じる
          </Button>
        </div>
      )}

      <div className="workspace">
        <ReturnThread proposal={state.appliedProposal} />
        <BriefWorkspace brief={state.brief} />
        <Conversation
          messages={state.messages}
          proposal={state.proposal}
          isSending={state.isSending}
          disabled={!agentAvailable}
          agentLabel={state.agentStatus.runtime?.label ?? "Agent"}
          onSend={send}
          onApply={applyProposal}
          onReject={rejectProposal}
        />
      </div>
    </div>
  );
}
