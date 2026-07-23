# 05. TiimoとOpenBriefの比較

## 結論

TiimoとOpenBriefは、どちらも実行機能の負荷を外部化し、「次に扱うもの」を少なく見せる点で似ています。しかし、支援する場所が異なります。

```text
外部情報を受け取る
    ↓
OpenBrief: 注意を配分し、応答状態を伝え、探索や集中から戻る
    ↓ 必要な行動だけを本人が確定
Planner: いつ実行するか決める
    ↓
Tiimo: Todoを予定へ変え、1件へ集中する
```

Tiimoは主に`Capture → Plan → Act`を扱う計画・実行支援アプリです。OpenBriefはその手前と周囲にある`Observe → Protect → Signal → Explore / Focus → Capture curiosity → Return`を扱います。

したがって、OpenBriefはTiimoの代替クローンではありません。Tiimoから学ぶのは、認知状態ごとの責務分離、粗い時間表現、Focusへの絞り込みです。独自に検証するのは、有限ブリーフ、Attention Signal、Curiosity Capture、Return Anchorによる注意遷移です。

## 比較の前提

比較対象の成熟度と根拠は同じではありません。

| 対象 | 根拠 | 現時点の状態 |
|---|---|---|
| Tiimo Android `1.1.4` | APK、Manifest、DEX、Hermes bytecodeの静的解析 | 配布済みアプリの観測結果。ただし動的挙動とbackend内部は未確認 |
| OpenBrief | 認知科学・HCI調査、製品仮説、GC-01 fixture | 設計・研究段階。効果を示すE1実証結果はまだない |

本文では、Tiimoについては静的解析で確認した範囲だけを記述します。OpenBriefについては「現在の設計」と「未検証仮説」を区別します。

## 一覧比較

| 観点 | Tiimo | OpenBrief |
|---|---|---|
| 中心課題 | やることを忘れず、時間へ配置し、実行する | 外部情報に注意を奪われても、義務と元作業を見失わずに戻る |
| 主な入力 | 本人が作るTodo、Activity、routine、外部calendar | 常駐Agentが正規化したGmail、RSSなどのObservation |
| 最初の判断 | 何をTodoとして残すか | 何を守るか、何を探索するか、何を無視するか |
| 中心entity | Todo、Activity、Checklist、Focus | Observation、Topic、DecisionCandidate、ProtectedIntent、AttentionSignal、CuriosityCapture、ReturnAnchor |
| 時間の扱い | Todoを時刻・時間帯のあるActivityへ変える | 必要な行動だけ予定案にし、明示確認後に外部calendarへ渡す |
| 実行支援 | Focus画面で今のActivityを1件へ絞る | Return Anchorで探索前の作業と次の一手を再提示する |
| 情報量の制御 | 週次PlanからFocusの1件へ絞る | 最大6 topic、残件数、推定時間、明示的終端を持つbriefにする |
| Captureの意味 | 実行候補をTodoとして保存する | 気になった問いを義務にせずCuriosity Captureへ退避する |
| AIの役割 | title/descriptionからchecklist、emoji、tag候補を作る | topic化、返信候補、表示理由、根拠、不明点、確信度を提示する |
| 人間の役割 | Todo作成、schedule、開始、pause、complete | 今日扱う、Signal共有、探索継続、Returnを決める |
| 自動化の境界 | Activity/Todo CRUD、同期、通知を製品内で所有する | 収集・提案は自動化し、Signal・calendar書き込みは本人操作後だけ行う。返信・委任は自動化しない |
| 通知・状態共有 | Activityのlocal通知とremote push/in-app message | 有限brief、Returnの手掛かり、expiration付きSlack status。強制blockは行わない |
| 設計上の成果 | Todo、予定、Focusを接続する。実際の製品効果は静的解析から判断できない | 探索超過と意図の見落としを減らし、集中中の応答状態を周囲へ伝えることを仮説として検証する |
| 評価 | APKから製品効果は判断できない | 探索超過、Return、着想価値、自律性、復帰予測、privacy discomfortを比較検証する |
| 実装状態 | React Native + Expoと複数backend境界を確認 | 実装未着手。現在は研究文書とGC-01 fixtureがcontract |

## 共通点

1. Todo、Activity、Focusや、Protect、Signal、Explore、Returnのように認知状態を段階へ分ける
2. 全情報を同時に見せず、現在の判断または操作へ表示を絞る
3. checklistやnextActionにより、再開・着手時の最初の操作を小さくする
4. AI出力をentityの最終決定ではなく、人間が採否を決める候補として扱う
5. entityの最終状態だけでなく、開始や完了など意味のある操作eventを持つ

## 認知フローの違い

### Tiimo: Capture → Plan → Act

Tiimoでは、認知負荷を次の3場面へ分けています。

```text
Todo
何を忘れたくないか
    ↓ schedule
Activity
いつ、どの程度やるか
    ↓ start
Focus
今は何をするか
```

Todoは未確定の実行候補、Activityは日時や時間帯を持つ予定、Focusは今取り組むActivityです。正確な時刻を決められない場合も、朝・昼・夜・いつでもという粗い配置を正式な状態として扱います。

### OpenBrief: Observe → Protect → Signal → Explore / Focus → Return

OpenBriefでは、情報接触の前後を次の場面へ分けます。

```text
Observation
何が起きたか
    ↓ topic化・根拠保持
Protect
見失いたくない意図は何か
    ↓ 本人が確定
Signal
応答状態と復帰予定を周囲へ伝えるか
    ↓ 本人が共有を選択
Explore / Focus
有限の情報から何を知るか、何へ集中するか
    ↓ 問いだけ退避
Return
探索前のどこへ戻るか
```

Protectは義務の完了を要求しません。SignalもExploreやFocusへの入場券ではありません。本人が守ると決めたものだけをProtectedIntentにし、必要なら外部plannerへ予定案として渡します。Signalを選んだ場合だけ、expiration付きのavailabilityをSlackなどへ出力します。

## 同じ言葉でも役割が異なる

### Capture

TiimoのTodo Captureは「後で実行するもの」を保存します。OpenBriefのCuriosity Captureは「気になる問い」を保存します。

Curiosity Captureへ期限、優先度、見積時間、未処理badgeを付けると、好奇心が新しい義務へ変わります。そのため、本人が昇格させるまでTodoやProtectedIntentにはしません。

### FocusとReturn

TiimoのFocusは、アプリ内の予定から今の1件を選び、countdown、checklist、進捗、pause、completeを提供します。

OpenBriefのReturnは、新しい作業を選ぶ機能ではありません。探索前に行っていた外部の作業について、再開点と次の物理的操作を戻します。また、`元の作業へ戻る`を押したことと、対象contextで実際に操作を再開したことを別eventとして扱います。

### AI支援

Tiimoで観測したAI境界は、既に作ると決めたActivityのchecklistや、emoji/tag候補を生成する入力支援です。

OpenBriefのAIは、行動を決める前の情報整理を担います。候補にはsource、時刻、表示理由、根拠、不明点、確信度を付けます。重要度や返信要否の最終決定、メール送信、calendar登録は本人に残します。

## ドメイン対応表

次は同一entityへの置き換えではなく、責務の近さを示す対応です。

| Tiimo | OpenBrief | 関係 |
|---|---|---|
| Todo | DecisionCandidate / ProtectedIntent | 行動候補という点は近いが、OpenBriefでは本人の確定前後を分ける |
| Activity | 確認済みcalendar proposal | 時間へ配置する点は近いが、OpenBriefはplanner全体を所有しない |
| Focus | ReturnAnchor / ReturnOutcome | 注意を1件へ戻す点は近いが、OpenBriefは実行UIを提供しない |
| Checklist item | ReturnAnchor.nextAction | 次の操作を小さくする点だけを引き継ぐ |
| Action history | Brief/Decision/Return event | 最終状態だけでなく意味のある遷移を記録する |
| 該当なし | Observation / Topic | 外部情報を根拠付きで束ねるOpenBrief固有の境界 |
| 該当なし | CuriosityCapture | タスク化しない問いの退避先 |

## 認識済みタスクの扱い

OpenBriefは、認識済みタスクを内部Todoとして複製しません。外部タスク管理をsource of truthとし、注意判断に必要な参照だけを持ちます。

```text
Todoist / GitHub / Calendar / Tiimo型planner
        │ タスク本体を所有
        ↓
ExternalTaskRef
        ↓ なぜ今見るかを提示
DecisionCandidate
        ↓ 本人が今日守ると確定
ProtectedIntent
        ↓
ReturnAnchor
```

| 外部タスク管理が所有 | OpenBriefが所有 |
|---|---|
| title、期限、完了、繰り返し、subtask | 表示理由、今日守るか、次の一手、戻るcontext |
| 正式なタスク状態 | 最終同期時点のstatus snapshot |
| schedule・completeの永続化 | 本人が確認する変更proposal |

OpenBrief上の操作は次の4つに限定します。

1. `今日守る`: 最大3件のProtectedIntentへ置く
2. `今やる`: Return Anchorを作り、元サービスのタスクを開く
3. `時間を確保`: 予定案を作り、確認後だけ外部へ書き込む
4. `今は扱わない`: 元タスクを変更せず、現在のbriefから閉じる

最小の参照モデルは次です。

```ts
type ExternalTaskRef = {
  source: 'github' | 'calendar' | 'todoist' | 'other'
  externalId: string
  url?: string
  statusSnapshot: 'open' | 'done' | 'unknown'
  dueAt?: string
  lastSyncedAt: string
}
```

CuriosityCaptureはこのモデルへ自動変換しません。本人が`タスクにする`を選んだ場合だけ外部サービスへhandoffし、作成されたExternalTaskRefを保存します。

## 技術アーキテクチャの違い

### Tiimoで確認した構成

```text
React Native / Expo client
├── Expo Router
├── Zustand: 端末・UI状態
├── TanStack Query: server状態
├── Zod + React Hook Form: 入力と変換
├── SecureStore: token
└── Notifications / Audio / Haptics
        ↓
Main API / Auth API / AI helper
```

Activity、Todo、Profile、質問、AI helperをserverへ同期し、Focusのcountdownや感覚feedbackは端末で処理します。Sentry、Mixpanel、Braze、AppsFlyer、RevenueCatなどの製品運用SDKも観測しました。

### OpenBriefの現在の境界

```text
Gmail / RSS
    ↓ 常駐Agent・CLIが読み取り、共通形式へ変換
Observation
    ↓ OpenBriefが検証・分類・topic化
DecisionCandidate / Topic
    ↓ 本人が選択
ProtectedIntent / CuriosityCapture / ReturnAnchor
```

現時点で確定している実装contractは、正規化済みObservationを入力にする[GC-01 fixture](../../../fixtures/golden-cases/gc-01-gmail-rss-return.json)です。UI framework、DB、deployment、認証方式はまだ決定していません。

OpenBriefではsource本文を信頼できないdataとして扱います。収集・要約Agentには、メール送信、calendar書き込み、shell実行権限を与えません。外部への書き込みは別の権限境界に置き、本人の明示確認を必要とします。

## Tiimoの観測から参考にする一般パターン

1. 認知状態ごとに画面とentityの責務を分ける
2. 正確な時刻だけでなく、朝・昼・夜・いつでもという粗い時間状態を持つ
3. UI状態、server状態、機密tokenを別の保存境界へ置く
4. countdownや感覚feedbackは端末、ownershipや永続化はserverという責務分離を使う
5. 最終状態だけでなく、開始・pause・完了など意味のあるeventを記録する

これらは一般的な設計原則として独自に実装します。Tiimo固有のコード、文言、asset、画面配置、API contractは使用しません。

## OpenBriefで変更する設計

1. CaptureしたものをすべてTodoにしない
2. 週次plannerやroutine管理を中核機能にしない
3. Focusを閉じた実行画面ではなく、元contextへのReturnとして扱う
4. AIをtask enrichmentだけでなく、根拠付きの情報整理へ使う
5. 完了数や滞在時間ではなく、見落とし、探索超過、Return、着想価値を測る

overdue、延長、overrideを責めないことや、好奇心を義務の報酬にしないことは、Tiimoの実装事実ではなくOpenBriefが置く独自の規範的要件です。

## MVPで採用しないもの

- 複数Profile、複数Todo list、custom recurrence
- 課金、paywall、entitlement、長いonboarding
- marketing attribution、engagement push、複数analytics SDK
- Tiimo互換API、既存hostへの接続、traffic replay
- TiimoのUI、文言、icon、色、animationの模倣

OpenBriefの中核仮説を検証する前にこれらを実装すると、planner製品の再構築へscopeが広がり、Attention Triageの成否が分からなくなります。

## 研究上の違い

TiimoのAPKからは、機能の存在と実装境界を観測できます。しかし、ADHD症状、生活の質、タスク実行率などへの効果は判断できません。analytics eventが存在しても、製品効果の証拠にはなりません。

OpenBriefは、次の主張を仮説として明示的に反証します。

- 有限briefが探索超過を減らすか
- Curiosity Captureが好奇心を損なわず終端到達を助けるか
- Return Anchorが実際の作業復帰を改善するか
- Protectが返信や期限の見落としを減らすか
- 追加機構が同じ内容の通知batchを上回るか

この比較はOpenBriefの有効性を示すものではありません。OpenBrief固有の効果は、N-of-1と比較研究によって初めて評価します。

## 直近の実装判断

Tiimo型のplanner全体を作る前に、GC-01だけを再生できるlocal prototypeを作ります。

```text
fixtureを読む
    ↓
Anchor Check
    ↓
Return Anchor
    ↓
Finite Brief
    ↓
Curiosity Capture
    ↓
Closure and Return
```

このvertical sliceでは、Gmail、RSS、LLM、認証、同期、課金へ接続しません。5画面の状態遷移と研究計測が成立してから、RSS、Gmailの順にsourceを追加します。

## 根拠文書

### Tiimo

- [静的解析レポート概要](../../reverse-engineering/tiimo/README.md)
- [クライアント・アーキテクチャ](../../reverse-engineering/tiimo/02-client-architecture.md)
- [ADHD支援機能とUX](../../reverse-engineering/tiimo/03-features-and-ux.md)
- [ネットワーク・バックエンド境界](../../reverse-engineering/tiimo/04-network-and-backend.md)
- [セキュリティ・プライバシー・解析限界](../../reverse-engineering/tiimo/06-security-privacy-and-limitations.md)

### OpenBrief

- [認知科学・HCIの研究基盤](01-research-foundations.md)
- [製品モデルと検証仮説](02-product-model-and-hypotheses.md)
- [Gmail＋RSSゴールデンケース](03-golden-case.md)
- [評価プロトコル](04-study-protocol.md)
- [GC-01 fixture](../../../fixtures/golden-cases/gc-01-gmail-rss-return.json)
