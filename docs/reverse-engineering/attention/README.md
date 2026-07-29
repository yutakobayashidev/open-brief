# Attention macOS静的解析レポート

## まず読む

OpenBriefのActivity Recall実装へ反映する場合は、最初に[Captureとcontext取得](03-capture-and-context-pipeline.md)、次に[OpenBriefへの採用判断](05-openbrief-adoption.md)を読んでください。Agent memoryを検討する場合は[AI Agent連携](07-agent-integration.md)を続けて参照してください。

このレポートは、ユーザーがGhidraへ読み込んだAttentionのmacOSバイナリを静的解析し、画面・Accessibility・時系列metadataを扱う一般的な設計を独立実装するための学びを整理したものです。Attentionのソースコード復元、転載、互換clientの作成を目的としません。

## 一文で言うと

Attentionは単純な定期スクリーンショット保存器ではない。

> ScreenCaptureKitによるcapture、差分OCR、Accessibility tree、app/domain単位のsegment、順序を守るbounded write queue、HEICから動画への後段compactionを分離したmacOS向けactivity timeline

OpenBriefが借りるべきなのは、画像を全保存する製品構造ではなく、capture trigger、privacy判定、context enrichment、順序付きcommitを別の責務として扱う設計である。

## 製品context

ユーザーから提供された2026-07-29時点の利用談では、製品群は少なくとも二つの価値へ説明されている。

- `Coast Local`: 個人のlocal screen memory。Claude CodeやDevin CLIがCoast CLIを検索し、本人とAgentの過去作業を確認する。
- `Attention` cloud: 組織へwork contextとenterprise-level insightを提供する。

具体例として、Zoomで見たpresentationを後からAgentがrecording内のframeから探し、slideをcropしてPDFへ再構成した事例が挙げられている。また、入力中のform、refreshで消えたdraft、過去に見たweb pageを検索できるsystem of recordとして説明されている。

これは提供者自身の利用談であり、独立した効果検証ではない。ただし、binaryで確認したCoast CLI、frame/video、OCR/FTS、FrameExtractorが、単なるtimeline再生ではなくAgent向けmemoryとartifact recoveryを支える理由を説明するproduct contextとして有用である。

## 最重要の発見

1. captureは一定間隔で起動するが、同時実行数とwrite queueに上限があり、過負荷時は古い仕事を溜めずframeをdropする。
2. focused app/windowと除外ruleをcapture前に判定し、除外appではAccessibility observationも停止する。
3. OCRは前frameとの差分を利用し、画面全体の再OCRを避ける。
4. Accessibility treeはframeごとの巨大blobだけでなく、hash付きnode、edge、snapshotに分離する新しいschemaを持つ。
5. frameのtimestamp順序を守るため、in-flight予約、遅延capture拒否、watchdog、bounded write queueを持つ。
6. 直近frameはHEIC画像として保存し、後からFFmpegで動画へまとめて`image_path`を外す二段階storageを採る。
7. timelineのsegment境界は少なくともapplication、domain、URLの変化から作られる。
8. Agent連携は、local CLI bridge、Agent用skill、外部Agentへのprompt routingを分離している。

## 文書一覧

| 文書 | 内容 |
|---|---|
| [01 Analysis scope](01-analysis-scope.md) | 対象、解析方法、確度、再現性の限界 |
| [02 Architecture](02-architecture.md) | module境界と全体data flow |
| [03 Capture and context](03-capture-and-context-pipeline.md) | capture、OCR、Accessibility、privacy trigger |
| [04 Storage, search, retention](04-storage-search-retention.md) | SQLite/GRDB schema、FTS、segment、compaction |
| [05 OpenBrief adoption](05-openbrief-adoption.md) | 採用、保留、不採用と実装順序 |
| [06 Security and limitations](06-security-privacy-and-limitations.md) | privacy境界、クリーンルーム方針、静的解析の限界 |
| [07 AI Agent integration](07-agent-integration.md) | CLI bridge、Agent skill、`/agent` routingと採用判断 |
| [08 Further analysis map](08-further-analysis-map.md) | 未調査surface、解析可能な問い、優先順位 |
| [09 Browser privacy](09-browser-privacy-path.md) | browser別URL取得、private判定、unreadable health |
| [10 Usage and sessions](10-usage-and-session-semantics.md) | frame count時間とtimestamp sessionの異なる意味 |
| [11 Sync, upload, airgap](11-sync-upload-airgap.md) | off-device pipeline、consent、DB guard、airgap |
| [12 Search and retrieval](12-search-retrieval-pipeline.md) | FTS query grammar、streaming、多段dedup、degradation |
| [Evidence](evidence/observations.md) | symbol、文字列、SQL、logから得た観測一覧 |

## 確度の表記

- **確認**: binary内のclass名、framework、SQL、設定key、log文字列から直接確認
- **強い推定**: 複数の確認事項が同じ処理flowを示すが、制御flow全体は復元できていない
- **推定**: 名前または一つの観測から導いた仮説
- **提案**: OpenBrief向けの独自設計。Attentionの実装事実ではない

## OpenBriefへの短い結論

MVPの5分captureをAttention型の常時録画へ変更しない。次だけを先に採用する。

```text
niri foreground event
    ↓ capture前のdeny判定
5分tick → bounded lane 1
    ↓
memory上のscreenshot → LM Studio
    ↓ raw imageは破棄
timestamp順にmetadataとsummaryだけcommit
```

MVP後にcapture頻度を上げる場合は、Attentionの差分OCR、in-flight予約、write ordering、Accessibility tree差分化を再評価する。Agentによるartifact recoveryまで扱う場合は、summary-only storeとは別に、明示opt-inの短期Evidence Storeを検証する。
