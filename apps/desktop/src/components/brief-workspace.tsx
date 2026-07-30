import type { Brief, BriefItem, Exploration } from "../domain";
import { Eyebrow, Surface } from "./ui";

function SourceStamp({
  source,
  observedAt,
}: Pick<BriefItem, "source" | "observedAt">) {
  return (
    <span className="source-stamp">
      {source} · {observedAt}
    </span>
  );
}

function ProtectItem({ item }: { item: BriefItem }) {
  return (
    <article className="brief-item brief-item--protect">
      <span className="brief-item__mark" aria-hidden>
        !
      </span>
      <div>
        <h3>{item.title}</h3>
        <p>{item.reason}</p>
        <SourceStamp {...item} />
      </div>
    </article>
  );
}

function ExploreItem({ item }: { item: Exploration }) {
  return (
    <article className="brief-item brief-item--explore">
      <div className="brief-item__duration">
        <strong>{item.minutes}</strong>
        <span>min</span>
      </div>
      <div>
        <h3>{item.title}</h3>
        <p>{item.reason}</p>
        <SourceStamp {...item} />
      </div>
    </article>
  );
}

export function BriefWorkspace({ brief }: { brief: Brief | null }) {
  if (!brief) {
    return (
      <main className="brief-panel" aria-busy>
        <div className="brief-skeleton" />
        <div className="brief-skeleton brief-skeleton--wide" />
        <div className="brief-skeleton" />
      </main>
    );
  }

  const isEmpty = brief.protect.length === 0 && brief.explore.length === 0;

  return (
    <main className="brief-panel" aria-labelledby="brief-heading">
      <header className="brief-header">
        <div>
          <Eyebrow>Today · {brief.generatedAt}</Eyebrow>
          <h1 id="brief-heading">見失わないための、今回分。</h1>
        </div>
        <div className="coverage" aria-label="収集状況">
          {brief.coverage.map((item) => (
            <span key={item.source}>
              <i className={`coverage__dot coverage__dot--${item.status}`} />
              {item.source} {item.observedAt}
            </span>
          ))}
        </div>
      </header>

      {isEmpty && (
        <Surface className="finite-stop">
          <span className="finite-stop__line" />
          <div>
            <strong>今回分はまだありません</strong>
            <p>Observationを投入して、Codexに一文送ると有限Briefを提案します。</p>
          </div>
          <span className="finite-stop__count">0</span>
        </Surface>
      )}

      <section className="brief-section" aria-labelledby="protect-heading">
        <div className="section-heading">
          <h2 id="protect-heading">先に守る</h2>
          <span>{brief.protect.length}件</span>
        </div>
        <div className="brief-list">
          {brief.protect.map((item) => (
            <ProtectItem key={item.id} item={item} />
          ))}
        </div>
      </section>

      <section className="brief-section" aria-labelledby="explore-heading">
        <div className="section-heading">
          <h2 id="explore-heading">寄り道してよい</h2>
          <span>{brief.explore.length}件まで</span>
        </div>
        <div className="brief-list">
          {brief.explore.map((item) => (
            <ExploreItem key={item.id} item={item} />
          ))}
        </div>
      </section>

      {!isEmpty && (
        <Surface className="finite-stop">
          <span className="finite-stop__line" />
          <div>
            <strong>今回分は以上です</strong>
            <p>新しい候補は、次のBriefまで増やしません。</p>
          </div>
          <span className="finite-stop__count">
            {brief.protect.length + brief.explore.length}
          </span>
        </Surface>
      )}
    </main>
  );
}
