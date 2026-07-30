import type {
  AgentStatus,
  Brief,
  DesktopEvent,
  TriageProposal,
} from "./domain";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface DesktopPort {
  connect(listener: (event: DesktopEvent) => void): () => void;
  loadBrief(): Promise<Brief>;
  sendMessage(text: string): Promise<void>;
  applyProposal(proposalId: string): Promise<void>;
  rejectProposal(proposalId: string): Promise<void>;
  retryReadiness(): Promise<AgentStatus>;
  authenticate(methodId: string): Promise<AgentStatus>;
}

export class TauriDesktopPort implements DesktopPort {
  connect(listener: (event: DesktopEvent) => void) {
    let connected = true;
    let unlisten: UnlistenFn | undefined;

    void listen<DesktopEvent>("desktop-event", (event) => {
      if (connected) listener(event.payload);
    })
      .then(async (stopListening) => {
        unlisten = stopListening;
        const anchor = await invoke<TriageProposal | null>("load_return_thread");
        if (anchor && connected) {
          listener({ type: "proposal_applied", proposal: anchor });
        }
        const status = await invoke<AgentStatus>("agent_start");
        if (connected) {
          listener({ type: "agent_status_changed", status });
        }
      })
      .catch((error: unknown) => {
        listener({
          type: "error",
          message: error instanceof Error ? error.message : String(error),
        });
      });

    return () => {
      connected = false;
      unlisten?.();
    };
  }

  loadBrief() {
    return invoke<Brief>("load_brief");
  }

  sendMessage(text: string) {
    return invoke<void>("agent_prompt", { text });
  }

  applyProposal(proposalId: string) {
    return invoke<void>("apply_proposal", { proposalId });
  }

  async rejectProposal() {}

  retryReadiness() {
    return invoke<AgentStatus>("agent_start");
  }

  authenticate(methodId: string) {
    return invoke<AgentStatus>("agent_authenticate", { methodId });
  }
}

export const fixtureBrief: Brief = {
  protect: [
    {
      id: "mail-thursday",
      title: "木曜の打ち合わせ時間を返す",
      reason: "先方が今日中の返答を待っています",
      source: "Gmail",
      observedAt: "12:27",
    },
  ],
  explore: [
    {
      id: "memory-article",
      title: "Agent memoryの記事を読む",
      reason: "いまの認証設計に使える実装例が含まれています",
      source: "RSS",
      observedAt: "12:25",
      minutes: 8,
    },
    {
      id: "release-notes",
      title: "Codexの更新点を確認する",
      reason: "ACPの接続方式に関係する変更があります",
      source: "RSS",
      observedAt: "11:58",
      minutes: 5,
    },
  ],
  coverage: [
    { source: "Gmail", observedAt: "12:27", status: "fresh" },
    { source: "RSS", observedAt: "12:25", status: "fresh" },
  ],
  generatedAt: "12:31",
};

const fixtureProposal: TriageProposal = {
  id: "proposal-1",
  summary: "メールを先に守り、memoryの記事を8分だけ探索します。",
  protectIds: ["mail-thursday"],
  exploreId: "memory-article",
  returnAnchor: "認証テストへ戻る",
  returnCommand: "cargo test -p openbrief-app auth",
};

export class FixtureDesktopPort implements DesktopPort {
  private listener: ((event: DesktopEvent) => void) | undefined;
  private timers = new Set<number>();

  connect(listener: (event: DesktopEvent) => void) {
    this.listener = listener;
    this.emit({
      type: "agent_status_changed",
      status: {
        availability: { status: "checking" },
        authentication: { status: "unknown" },
        process: { status: "stopped" },
        runtime: null,
      },
    });
    this.later(450, () => {
      this.emit({
        type: "agent_status_changed",
        status: fixtureAgentStatus,
      });
    });

    return () => {
      this.listener = undefined;
      this.timers.forEach((timer) => window.clearTimeout(timer));
      this.timers.clear();
    };
  }

  async loadBrief() {
    return fixtureBrief;
  }

  async sendMessage(text: string) {
    const id = `agent-${Date.now()}`;
    this.emit({ type: "message_started", id, role: "agent" });
    const reply = text.includes("木曜")
      ? "了解しました。木曜のメールを今日守る項目に残し、memoryの記事を8分の探索として提案します。戻り先は認証テストです。"
      : "判断を有限Briefに反映する提案を作りました。確定するまでは保存しません。";
    const chunks = reply.match(/.{1,12}/gu) ?? [reply];
    chunks.forEach((chunk, index) => {
      this.later(180 * (index + 1), () => {
        this.emit({ type: "message_delta", id, text: chunk });
      });
    });
    this.later(180 * (chunks.length + 1), () => {
      this.emit({ type: "message_finished", id });
      this.emit({ type: "proposal_received", proposal: fixtureProposal });
      this.emit({ type: "turn_finished" });
    });
  }

  async applyProposal(proposalId: string) {
    if (proposalId !== fixtureProposal.id) {
      throw new Error("提案が見つかりません");
    }
    this.emit({ type: "proposal_applied", proposal: fixtureProposal });
  }

  async rejectProposal() {
    this.emit({ type: "turn_finished" });
  }

  async retryReadiness() {
    this.emit({
      type: "agent_status_changed",
      status: {
        ...fixtureAgentStatus,
        authentication: { status: "unknown" },
        process: { status: "starting" },
      },
    });
    return fixtureAgentStatus;
  }

  async authenticate() {
    return fixtureAgentStatus;
  }

  private emit(event: DesktopEvent) {
    this.listener?.(event);
  }

  private later(delay: number, callback: () => void) {
    const timer = window.setTimeout(() => {
      this.timers.delete(timer);
      callback();
    }, delay);
    this.timers.add(timer);
  }
}

const fixtureAgentStatus: AgentStatus = {
  availability: { status: "available" },
  authentication: { status: "authenticated" },
  process: { status: "ready" },
  runtime: {
    providerId: "codex",
    label: "Codex",
    source: "packaged",
    version: "1.1.7",
    path: "/fixture/libexec/openbrief/codex-acp",
  },
};
