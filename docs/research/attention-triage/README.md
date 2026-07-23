# Attention Triage研究

## まず読む

OpenBriefの目標は、好奇心を抑えることではありません。

> 知りたいことを有限の探索へ変え、探索前に持っていた意図へ戻れるようにする。

ユーザーはニュースから着想を得たい一方、無限スクロールへ入ると、返信、締切、中断中の作業が見えなくなることがあります。OpenBriefは情報を禁止せず、探索の境界、好奇心の退避先、元の作業へ戻る手掛かりを提供します。

## 60秒で分かる構想

```text
Gmail / RSS
    ↓ 常駐Agentが読み取り専用で収集
Observation Inbox
    ↓ 重複排除・トピック化・根拠保持
Protect: 見失いたくない意図を最大3件だけ確認
    ↓
Signal: 応答状態と復帰予定を本人確認後に周囲へ共有
    ↓
Explore / Focus: 最大6件の有限ブリーフ、または集中作業
    ↓
Capture: 気になった問いを義務にせず退避
    ↓
Return: 探索前の作業と次の一手を再提示
```

最初の研究質問は次です。

> 有限ブリーフ、好奇心の退避、再開手掛かりは、着想価値と自律性を維持しながら、予定外の探索延長と義務の見落としを減らせるか。

## 文書一覧

| 文書 | 読む目的 |
|---|---|
| [01 Research foundations](01-research-foundations.md) | 認知科学・HCIの根拠と限界を確認する |
| [02 Product model](02-product-model-and-hypotheses.md) | 製品仮説、設計原則、反証条件を確認する |
| [03 Golden case](03-golden-case.md) | Gmail＋RSSを使う理想的な1セッションを共有する |
| [04 Study protocol](04-study-protocol.md) | N-of-1と小規模比較研究を実施する |
| [05 Tiimo comparison](05-tiimo-comparison.md) | 静的解析したTiimoとOpenBriefの共通点・相違点を確認する |
| [06 Objective assessment](06-objective-assessment.md) | 構想の強み、kill risk、継続・停止条件を確認する |
| [GC-01 fixture](../../../fixtures/golden-cases/gc-01-gmail-rss-return.json) | ゴールデンケースを実装・テスト用の入力と期待状態として使う |

## 根拠の読み方

本文では、研究の質とOpenBriefへの適用距離を混同しないため、主張を次のtierへ分けます。

| 表記 | 意味 |
|---|---|
| E1 | OpenBrief自体を対象にした実証結果。現時点では存在しない |
| E2 | 近接する認知機構やUIを扱った実験・field study |
| E3 | 観察、自己報告、質的研究、review、間接的な知見 |
| H | OpenBrief固有の未検証仮説 |
| R | プライバシー、自律性、安全性から置く規範的要件 |

「ADHDなら必ずこうなる」「無限スクロールはドーパミン依存を起こす」「hyperfocusはADHDの中核症状」とは断定しません。診断の有無ではなく、探索中に以前の意図が見えなくなる行動パターンを対象にします。

## Oracleレビュー

2026-07-21にOracleからGPT-5.6 SolをExtra High reasoningで利用し、既存のUX、実装、セキュリティ文書を添付して第二モデルレビューを行いました。Oracleの回答は助言として扱い、本文の研究主張は一次論文または原著論文へのリンクで再確認しました。

Oracleと文献調査が一致した重要点は次です。

- 好奇心を義務達成の報酬にしない
- Curiosity CaptureをTodoへ自動変換しない
- ニュース閲覧前にReturn Anchorを残す
- 強制ロックではなく、本人が上書きできる摩擦を使う
- 単純な通知バッチングを上回るか検証する

## 現在の決定

- 義務系の最初のsource: Gmail
- 好奇心系の最初のsource: RSS / OPML
- Slack message input: Gmail＋RSSで中核仮説を検証した後に追加
- Slack status output: 最初のOutput Adapterとして、本人操作とexpiration付きで追加
- カレンダー書き込み: ユーザーの明示確認後だけ
- 自動返信、自動委任、強制ブロック: MVP対象外
