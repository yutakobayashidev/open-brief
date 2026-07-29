# OpenBrief

Androidアプリの静的解析と認知科学・HCI研究を、独立したAttention Triageアプリの設計へ変換するための調査リポジトリです。

## Architecture Decisions

- [ADR一覧](docs/adr/README.md)
- [ADR 0001: Local-firstなデータ境界とModel Gateway](docs/adr/0001-adopt-local-first-data-and-model-boundaries.md)
- [ADR 0002: Attention SignalとSlack Status Output](docs/adr/0002-adopt-attention-signals-and-slack-status-output.md)

## Tiimo調査レポート

- [調査概要と目次](docs/reverse-engineering/tiimo/README.md)
- [独自実装ブループリント](docs/reverse-engineering/tiimo/05-reimplementation-blueprint.md)

## Attention macOS静的解析

- [調査概要と目次](docs/reverse-engineering/attention/README.md)
- [Captureとcontext取得](docs/reverse-engineering/attention/03-capture-and-context-pipeline.md)
- [OpenBriefへの採用判断](docs/reverse-engineering/attention/05-openbrief-adoption.md)
- [AI Agent連携](docs/reverse-engineering/attention/07-agent-integration.md)
- [追加バイナリ解析マップ](docs/reverse-engineering/attention/08-further-analysis-map.md)
- [Browser privacy解析](docs/reverse-engineering/attention/09-browser-privacy-path.md)
- [Usageとsession semantics](docs/reverse-engineering/attention/10-usage-and-session-semantics.md)
- [Sync・upload・airgap解析](docs/reverse-engineering/attention/11-sync-upload-airgap.md)
- [Searchとretrieval pipeline](docs/reverse-engineering/attention/12-search-retrieval-pipeline.md)

## Attention Triage研究

- [研究概要と目次](docs/research/attention-triage/README.md)
- [Gmail＋RSSゴールデンケース](docs/research/attention-triage/03-golden-case.md)
- [TiimoとOpenBriefの比較](docs/research/attention-triage/05-tiimo-comparison.md)
- [構想の客観評価](docs/research/attention-triage/06-objective-assessment.md)
- [ADHD向けContext ResumptionとOracleレビュー](docs/research/attention-triage/07-adhd-context-resumption-oracle-review.md)
- [awesome-adhd横断レポート](docs/research/attention-triage/08-awesome-adhd-cross-report-synthesis.md)
- [Resume CueとWindow Transitionを比較するMVP](docs/research/attention-triage/09-window-transition-mvp-reset.md)
- [入力不要のActivity Recall Timeline MVP](docs/research/attention-triage/10-activity-recall-timeline-mvp.md)
- [qwen-audio-agent調査とaudio採用判断](docs/research/attention-triage/11-qwen-audio-agent-assessment.md)
- [GC-01実装fixture](fixtures/golden-cases/gc-01-gmail-rss-return.json)
- [GC-02 Activity Recall fixture](fixtures/golden-cases/gc-02-activity-recall-timeline.json)
- [GC-03 Activity Recall fail-closed fixture](fixtures/golden-cases/gc-03-activity-recall-fail-closed.json)
- [評価プロトコル](docs/research/attention-triage/04-study-protocol.md)

解析対象APKは `apks/com.tiimo.androidappreactnative/` に置かれています。APKや復元コードを配布・転載することを目的としていません。
