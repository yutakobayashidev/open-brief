import type { TriageProposal } from "../domain";
import { ArrowIcon, Button, Eyebrow } from "./ui";

export function ReturnThread({
  proposal,
}: {
  proposal: TriageProposal | null;
}) {
  const copyCommand = async () => {
    if (proposal) {
      await navigator.clipboard?.writeText(proposal.returnCommand);
    }
  };

  return (
    <aside className="return-thread" aria-labelledby="return-heading">
      <div className="return-thread__heading">
        <Eyebrow>Return thread</Eyebrow>
        <span className="return-thread__kept">常に保持</span>
      </div>

      <div className="thread">
        <div className="thread__step thread__step--past">
          <span className="thread__node" />
          <div>
            <span className="thread__label">いままで</span>
            <strong>{proposal ? "元の作業を保持中" : "再開点はまだありません"}</strong>
          </div>
        </div>
        <div className="thread__step thread__step--current">
          <span className="thread__node" />
          <div>
            <span className="thread__label">このBrief</span>
            <strong>守るものを決める</strong>
          </div>
        </div>
        <div className="thread__step">
          <span className="thread__node" />
          <div>
            <span className="thread__label">戻る場所</span>
            <h2 id="return-heading">
              {proposal?.returnAnchor ?? "triage後にここへ残ります"}
            </h2>
            {proposal && <code>{proposal.returnCommand}</code>}
          </div>
        </div>
      </div>

      <Button
        className="return-thread__action"
        onClick={copyCommand}
        disabled={!proposal}
      >
        戻るcommandをコピー
        <ArrowIcon />
      </Button>
      <p className="return-thread__hint">
        探索を閉じたら、ここから同じ文脈へ戻れます。
      </p>
    </aside>
  );
}
