# 07. ADHD向けContext ResumptionのOracleレビュー

## Status

- 調査日: 2026-07-27
- 対象: OpenBriefと`awesome-adhd`を接続した、Screenpipe型の受動context取得構想
- 第二モデル: Oracle経由のGPT-5.6 Sol
- reasoning: Extra High
- Oracle session: `openbrief-adhd-context-manual`
- 入力: 関連する研究、ADR、fixture、`awesome-adhd`の概念文書19件、約46,000 token
- 位置づけ: 実装判断のための助言。OpenBrief自体を対象にした実証結果ではない

## 結論

現在の判断は**Conditional Go**です。

```text
Resume Packという問題仮説                   Go
3週間の最小比較実験                         Go
最小限のapp-native観測                      Go
受動capture基盤                             Hold
always-on life-log                          No-Go
Screenpipe現行mainへの製品依存              No-Go
```

作るべきものは「ADHD向けに全画面と音声を記録するScreenpipe」ではありません。

> 中断時に失われる行動状態を、信頼可能な最小単位で次のsessionへ引き渡すAttention Handoff layerを検証する。

OpenBriefの前景は引き続き`Protect → Signal → Explore / Focus → Capture → Return`です。受動context取得は独立した目的ではなく、Return Anchorの入力負担を減らし、実際の復帰を支援・観測する背景機構として扱います。

## Oracleへ依頼した問い

Oracleには既存案への追認ではなく、次を独立に評価するよう依頼しました。

1. 最も強いBuild argumentとStop argument
2. ADHD-informedな製品として未充足なjob-to-be-done
3. Screenpipeを使う、使わない、外部接続する場合の比較
4. 3週間で反証可能なMVPとgolden case
5. 成功指標とguardrail
6. privacy、監視転用、医療的主張、ライセンス、誤推定のrisk
7. 暫定案の盲点と、より単純な代替案
8. Go、Hold、No-Go判断

Oracleの回答は助言として扱い、主な判断を[製品モデル](02-product-model-and-hypotheses.md)、[客観評価](06-objective-assessment.md)、[GC-01](../../../fixtures/golden-cases/gc-01-gmail-rss-return.json)、Screenpipe公式資料と突き合わせました。

## 最も重要な分解

画面やapp状態から復元しやすい情報と、本人の頭の中にしかない情報を混同しません。

| 種類 | 例 | 扱い |
|---|---|---|
| Observed | file、URL、cursor、test名、command、app switch | provenance付きの観測事実 |
| Inferred | 目的候補、次の一手候補 | AIまたはruleが作る訂正可能な候補 |
| User-confirmed | 本人が確定した目的と次の一手 | Return Anchorとして利用可能 |

画面から比較的確実に分かるのは「どこにいたか」です。「なぜそれをしていたか」「本当は次に何を判断するつもりだったか」は一意に決まりません。

したがって、Resume Packでは次のように表示を分けます。

```text
観測
  refresh.test.ts
  expired refresh token returns 401
  直近testは失敗

目的候補
  refresh token失敗時の認証testを修正

次の一手候補
  testを再実行し、現在のstatus codeを確認

[ここから再開] [修正] [今は戻らない]
```

AIの推定を観測事実として表示せず、推定できない場合は`不明`を許容します。

## 最も強いBuild argument

一般的なlife-logは大量の過去を検索可能にします。しかし、中断直後のユーザーには、検索語を考え、複数結果を比較し、正しい時点を探すこと自体が新しい負担になります。

Resume Packには、過去を次の一つの操作へ圧縮する価値がある可能性があります。

> 会議、通知、探索、別tabへの移動が終わったとき、履歴を読み直さず、直前の目的、作業位置、今できる最小の一手を数秒で取り戻す。

これは[Return Anchor仮説](02-product-model-and-hypotheses.md#3-探索前にreturn-anchorを残す)を、active contextの観測で補強する案です。

## 最も強いStop argument

取得dataを増やしても、画面に存在しない意図は復元できない可能性があります。

最大の中止条件は次です。

> 自動Resume Packが、5〜10秒で本人が残した一行メモと比べて、復帰率、復帰時間、入力負担のいずれも改善しない。

この場合、受動captureはprivacy、CPU、保存、誤推定、ライセンス、信頼のcostだけを追加します。

これは既存の客観評価が置く問いとも一致します。

> 単純な有限batchと一行の復帰手掛かりを超える複雑さに、本当に追加価値があるか。

## ADHD-informedなJob to be Done

初期対象は診断名ではなく、次の行動条件で定義します。

> 中断が終わったとき、履歴を探したり状況を思い出したりせず、直前の目的、作業位置、今できる最小の一手を、責められることなく数秒で取り戻したい。

このjob自体はADHDに固有ではありません。ADHD-informedである意味は、次の設計制約にあります。

- 中断前に毎回メモできることを前提にしない
- 検索語、分類、優先度を考えさせない
- 選択肢と視覚情報を増やさない
- 未完了、延長、戻らない選択を人格評価しない
- 次の一手を小さく具体化する
- 誤推定を一操作で訂正できる
- 経過時間と次の予定も同時に定位できる

「ADHDを改善する記憶AI」や治療効果は主張しません。初期の表現は`ADHD-informed task resumption`または「中断後の作業復帰を支えるlocal-first tool」とします。

## Screenpipeを使う3案

| 案 | 利点 | 最大の問題 | 判断 |
|---|---|---|---|
| 現行Screenpipeをforkまたは組み込む | richなcaptureを早く利用できる | 商用ライセンス、過剰取得、外部実装への中核依存 | 製品中核にはNo-Go |
| Screenpipeを使わず最小collectorを作る | 仮説に必要なdataと権限だけ扱える | appごとのadapterが必要 | MVPではGo |
| 外部Context Providerとして接続する | capture engineをOpenBriefから分離できる | API変更とライセンス上のintegration判断 | 研究後までHold |

Screenpipe現行mainはMITではなく、個人非商用、非営利、教育、研究などに無償利用を限定したsource-availableです。商用製品への組込み、配布、競合製品での利用には別契約が必要です。

- [Screenpipe公式README](https://github.com/screenpipe/screenpipe)
- [Screenpipe Commercial License](https://github.com/screenpipe/screenpipe/blob/main/LICENSE.md)

旧MIT版を検討する場合も、最後のMIT commitを固定し、それ以降の変更を混入させない管理が必要です。

`awesome-adhd`も2026-07-27時点ではライセンス未決定です。本メモでは調査結果を設計上の参考として扱い、文章や構造の再配布権があるとは仮定しません。

## 3週間の最小実験

### Research question

> 最小観測から生成したResume Packは、deep linkだけ、または本人による一行Return Anchorより、実質的な作業復帰を改善するか。

### 比較条件

同一参加者内でsessionをランダム化します。

| 条件 | 内容 |
|---|---|
| C0 Restore only | 元app、file、URLを開くdeep link |
| C1 Manual Anchor | 同じdeep linkと、本人が5〜10秒で入力した一行メモ |
| C2 Auto-Minimal Pack | 同じdeep linkと、最小観測から作った目的、再開点、次の一手候補 |

全条件でdeep link、通知timing、元contextを開く操作数を揃えます。そうしなければ、記憶支援ではなくnavigation短縮を測ることになります。

### 最小観測

- active application
- window title
- IDEのrepository、branch、file、cursor、選択範囲
- browserのURL、tab title、scroll位置
- terminalの直近commandとexit status
- calendar meetingの開始と終了
- 中断直前のapp switch

MVPでは次を取得しません。

- screenshot
- OCR
- microphoneとsystem audio
- 全key入力
- clipboard本文
- 長期の詳細timeline

raw eventは直前60〜90秒のrolling bufferへ置き、Pack生成と確認後に削除します。永続保存するのは本人が確認したPackと、その生成根拠を説明する最小provenanceだけとします。

## Golden Case: GC-RP-01

### ゴール

> 認証testの修正中に30分の会議へ入り、会議終了後5分以内に、意味のあるtest実行または編集へ戻れる。

既存[GC-01](../../../fixtures/golden-cases/gc-01-gmail-rss-return.json)のactive contextを流用します。

```text
現在の作業
  認証testの修正

再開点
  refresh tokenの失敗ケース

次の一手
  refresh失敗時に401を期待するtestを追加
```

### Session

1. 13:58、IDEで認証testを修正している
2. 14:00、Calendar上の会議開始により会議appへ移動する
3. 14:30、会議終了を確実なReturn triggerとして扱う
4. 条件に応じてRestore、Manual Anchor、Auto-Minimal Packを表示する
5. 対象contextを開いた後の継続と意味のある操作を観測する

### 受け入れ条件

- 正しいrepository、file、test位置を開ける
- Packの確認または修正が10秒以内
- 5分以内に意味のある編集またはtest実行が起きる
- 次の一手を推定できない場合は`不明`と表示する
- screenshot、音声、terminal全文を永続保存しない
- `今は戻らない`を失敗や人格評価として表示しない

### 必須failure cases

- password manager、banking、health関連appは観測対象外
- 複数projectを短時間に移動していた場合は自動確定しない
- 目的が画面から分からない場合は推測しない
- 会議後に緊急taskが発生した場合、古いPackを強制しない
- Pack生成に失敗しても、元appを開くRestoreは利用できる

## Metrics

### Primary outcome

中断終了から10分以内の`substantive return`率を使います。

```text
対象contextを開いた
かつ
2分以上作業を継続した、または意味のある編集・実行を行った
```

`ここから再開`を押しただけでは成功に数えません。

### Secondary outcomes

- 最初の意味ある操作までの時間
- 正しいfile、URL、taskへ戻れた割合
- 目的、再開点、次の一手ごとの訂正率
- Packの確認と修正に要した時間
- Packを無視した割合
- 中断前後に別taskへ移った割合
- privacy discomfortと監視感
- 手動Anchorと自動Packの選好
- 4週間継続したいか

## 暫定Continue gates

次は科学的に確立された閾値ではなく、過剰投資を防ぐための事前判断基準です。

- C2がC0より`substantive return`率を15 percentage point以上改善する、またはmedian return latencyを25%以上短縮する
- C2がC1と同等以上のReturnを維持し、入力と修正時間を30%以上減らす
- Pack全体のmedian確認時間が10秒以内
- 誤ったtaskへ実際に誘導したsessionが2%以下
- 高損失の誤誘導が0件
- 機微情報の永続保存事故が0件
- 参加者の過半数がC1またはC2をC0より選ぶ

## Stop gates

- C2がC1を上回らない
- Packの訂正率が20〜25%を継続して超える
- 訂正と確認時間が、削減した復帰時間を上回る
- rich captureが最小観測より改善しない
- privacy discomfortによりreal data条件で使われない
- Resume Packを見ても着手や回避が変わらない
- Packが新しい未処理Inboxになる

## 主要riskとguardrail

| Risk | Guardrail | No-Go条件 |
|---|---|---|
| Privacy | allowlist、raw短期保持、画像と音声なし、即時削除 | 機微画面の永続保存 |
| 監視転用 | 個人専用、remote dashboardなし、管理者APIなし、生産性scoreなし | 雇用評価、出席、稼働監視への利用 |
| 医療的主張 | 行動条件で対象化し、診断、治療、症状検出を主張しない | ADHDを治す、症状を検出するという表示 |
| 誤推定 | field別confidence、根拠表示、`不明`、一操作訂正 | 高損失の誤誘導 |
| 第三者data | 音声と会議内容を取得しない、除外appを設ける | 同意できない会話や顧客情報の記録 |
| local端末侵害 | OS権限分離、短期retention、secret除外 | raw storeを他processが容易に読める |
| backlog化 | 最新1〜3件だけを表示し、未処理件数を義務表示しない | badgeや罪悪感の増加 |

local-firstは、それ自体ではprivacy保証になりません。[ADR 0001](../../adr/0001-adopt-local-first-data-and-model-boundaries.md)の権限、egress、secret、retention境界を維持します。

## 現時点で採用する最小baseline

自動Resume Packを評価する前に、5秒Return Hotkeyをbaselineとして実装候補に残します。

```text
[global hotkey]

次は何をしますか？
[一行入力]
```

現在のfile、URL、cursorだけを自動添付します。この条件が十分に効き、自動Packが上回らない場合は、これ自体を完成形として扱います。

もう一つの代替は、汎用screen captureではなく、IDEやbrowserから構造化されたResume Candidateを受け取るapp-native handoffです。

```text
goalHint
resource
position
lastAction
nextActionCandidate
provenance
```

開発者や研究者を初期対象にする場合、汎用captureより精度、privacy、実装範囲のbalanceが良い可能性があります。

## 未解決事項

- 確実な中断triggerをcalendar meeting、lock/unlock、global hotkey、Focus/Explore開始のどこまで含めるか
- `meaningful operation`をappごとにどう定義するか
- 目的候補をrulesだけで作るか、local modelを使うか
- raw rolling bufferをmemoryだけに置けるか
- IDE、browser、terminalのどれを最初のgolden pathにするか
- Screenpipeをshadow comparisonへ使う場合の書面上の利用条件
- 記憶支援ではなく不安や回避が原因だった場合に、別介入として何を検証するか

## 次の判断

実装前に、次だけを1ページへ固定します。

1. C0、C1、C2の条件
2. 計測eventと`substantive return`の判定
3. Continue gateとStop gate
4. GC-RP-01のfixture
5. raw dataの取得、保持、削除境界

この比較で自動Resume Packが一行Anchorを上回った場合だけ、Screenpipe接続、OCR、音声、MCP、長期履歴、複数source統合を再検討します。
