import {
  useEffect,
  useRef,
  useState,
  type FormEvent,
  type KeyboardEvent,
} from "react";
import type { ConversationMessage, TriageProposal } from "../domain";
import { Button, Eyebrow, SendIcon } from "./ui";

function ProposalCard({
  proposal,
  onApply,
  onReject,
}: {
  proposal: TriageProposal;
  onApply: () => void;
  onReject: () => void;
}) {
  return (
    <section
      className="proposal-card"
      role="dialog"
      aria-modal="false"
      aria-labelledby="proposal-heading"
    >
      <div className="proposal-card__heading">
        <span className="proposal-card__signal" aria-hidden />
        <div>
          <span>変更の提案</span>
          <h3 id="proposal-heading">確定前に確認</h3>
        </div>
      </div>
      <p>{proposal.summary}</p>
      <dl>
        <div>
          <dt>探索後</dt>
          <dd>{proposal.returnAnchor}</dd>
        </div>
        <div>
          <dt>再開command</dt>
          <dd>
            <code>{proposal.returnCommand}</code>
          </dd>
        </div>
      </dl>
      <div className="proposal-card__actions">
        <Button size="small" onClick={onReject}>
          修正する
        </Button>
        <Button size="small" variant="primary" onClick={onApply}>
          この内容で確定
        </Button>
      </div>
    </section>
  );
}

export function Conversation({
  messages,
  proposal,
  isSending,
  disabled,
  agentLabel,
  onSend,
  onApply,
  onReject,
}: {
  messages: ConversationMessage[];
  proposal: TriageProposal | null;
  isSending: boolean;
  disabled: boolean;
  agentLabel: string;
  onSend: (text: string) => void;
  onApply: () => void;
  onReject: () => void;
}) {
  const [draft, setDraft] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    scrollRef.current?.scrollTo({
      top: scrollRef.current.scrollHeight,
      behavior: "smooth",
    });
  }, [messages, proposal]);

  const submit = (event?: FormEvent) => {
    event?.preventDefault();
    const text = draft.trim();
    if (!text || disabled || isSending) return;
    onSend(text);
    setDraft("");
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
      submit();
    }
  };

  return (
    <aside className="conversation" aria-labelledby="conversation-heading">
      <header className="conversation__header">
        <div>
          <Eyebrow>{agentLabel} sidecar</Eyebrow>
          <h2 id="conversation-heading">一文で整理する</h2>
        </div>
        {isSending && <span className="thinking">考え中</span>}
      </header>

      <div className="conversation__stream" ref={scrollRef} aria-live="polite">
        {messages.map((message) => (
          <article
            className={`message message--${message.role}`}
            key={message.id}
          >
            <span>{message.role === "agent" ? agentLabel : "あなた"}</span>
            <p>
              {message.text}
              {message.state === "streaming" && (
                <i className="stream-caret" aria-label="生成中" />
              )}
            </p>
          </article>
        ))}
        {proposal && (
          <ProposalCard
            proposal={proposal}
            onApply={onApply}
            onReject={onReject}
          />
        )}
      </div>

      <form className="composer" onSubmit={submit}>
        <label htmlFor="triage-message">このBriefをどう扱いますか？</label>
        <textarea
          id="triage-message"
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="木曜のメールは今日。記事は8分だけ見る。"
          rows={3}
          disabled={disabled}
        />
        <div className="composer__footer">
          <span>⌘ Enter で送信</span>
          <Button
            variant="primary"
            type="submit"
            aria-label="Agentへ送信"
            disabled={!draft.trim() || disabled || isSending}
          >
            <SendIcon />
          </Button>
        </div>
      </form>
    </aside>
  );
}
