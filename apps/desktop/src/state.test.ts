import { describe, expect, it } from "vitest";
import { appReducer, initialState } from "./state";

describe("appReducer", () => {
  it("streams agent text into one message", () => {
    const started = appReducer(initialState, {
      type: "message_started",
      id: "agent-1",
      role: "agent",
    });
    const first = appReducer(started, {
      type: "message_delta",
      id: "agent-1",
      text: "有限",
    });
    const second = appReducer(first, {
      type: "message_delta",
      id: "agent-1",
      text: "Brief",
    });

    expect(second.messages.at(-1)).toMatchObject({
      id: "agent-1",
      text: "有限Brief",
      state: "streaming",
    });
  });

  it("never applies a proposal before confirmation", () => {
    const proposal = {
      id: "proposal-1",
      summary: "8分だけ探索",
      protectIds: ["mail"],
      exploreId: "article",
      returnAnchor: "認証テストへ戻る",
      returnCommand: "cargo test",
    };
    const proposed = appReducer(initialState, {
      type: "proposal_received",
      proposal,
    });

    expect(proposed.proposal).toEqual(proposal);
    expect(proposed.appliedProposal).toBeNull();

    const applied = appReducer(proposed, {
      type: "proposal_applied",
      proposal,
    });
    expect(applied.proposal).toBeNull();
    expect(applied.appliedProposal).toEqual(proposal);
  });

  it("keeps failures actionable and ends the sending state", () => {
    const sending = { ...initialState, isSending: true };
    const failed = appReducer(sending, {
      type: "error",
      message: "Codexとの接続が切れました",
    });

    expect(failed.error).toBe("Codexとの接続が切れました");
    expect(failed.isSending).toBe(false);
  });

  it("keeps availability, authentication, and process state separate", () => {
    const status = {
      availability: { status: "available" as const },
      authentication: {
        status: "required" as const,
        methods: [{ id: "chatgpt", name: "ChatGPT" }],
      },
      process: { status: "stopped" as const },
      runtime: {
        providerId: "codex",
        label: "Codex",
        source: "nix_store" as const,
        version: "1.1.7",
        path: "/nix/store/example/libexec/openbrief/codex-acp",
      },
    };
    const updated = appReducer(initialState, {
      type: "agent_status_changed",
      status,
    });

    expect(updated.agentStatus).toEqual(status);
    expect(updated.agentStatus.authentication.status).toBe("required");
    expect(updated.agentStatus.process.status).toBe("stopped");
  });
});
