# 03. ADHD支援機能とUX

## 設計の中心

このアプリの価値は「多機能なカレンダー」ではなく、意思決定を3つの場面に分けることです。

| 場面 | ユーザーの問い | UI |
|---|---|---|
| Capture | 何を忘れたくないか | Todo |
| Plan | いつ・どの程度やるか | Activity / weekly plan |
| Act | 今なにをすればよいか | Focus |

独自版では、各画面に別の役割を持たせ、1画面ですべてを管理させません。

## Plan

### 確認した挙動

- 週単位でActivityを取得・表示
- 左右の週を先読み
- 日付と時間帯でActivityをグループ化
- Activityの作成、編集、削除、完了
- 単発・繰り返し、終日、いつでも、時間帯指定
- checklist付きActivity
- 外部カレンダー由来Activityの識別

時間タイプ:

- `attime`: 開始・終了時刻あり
- `allday`: 終日
- `anytime`: 時刻未指定
- `morning`, `day`, `evening`: 粗い時間帯

繰り返し:

- 毎日、平日、週末
- 毎週、隔週、毎月、毎年
- interval、曜日、月内日付、開始・終了日を持つcustom repeat

### ADHD向けの意味

正確な開始時刻を決められないタスクにも「朝」「昼」「夜」という置き場所があります。これは、完璧な予定作成を要求せず、曖昧な意図を少しだけ具体化する段階を提供します。

### 独自版での判断

MVPは`時刻指定 / 朝 / 昼 / 夜 / いつでも`だけにします。custom repeatと外部カレンダーは後回しにします。

## Todo

### 確認した挙動

- 複数Todoリスト
- Todoとsubtaskの作成、編集、削除、完了
- drag and dropによる並べ替え
- 完了セクションの表示切り替え
- TodoからActivityへの変換
- リスト単位のgroupingとsort設定

Todoの分類候補:

- Priority: High / Medium / Low
- Eisenhower: Do Now / Schedule / Delegate / Eliminate
- Duration: 15分未満 / 15〜30分 / 30〜60分 / 60分超
- Manual

Todoモデルにはtitle、notes、duration、色・icon、subtasks、完了状態、groupingがあります。

### ADHD向けの意味

優先度だけでなく「必要時間」で選べるため、現在の空き時間や実行可能性から次の候補を決められます。Todoを直接カレンダーへ変換できるため、収集と計画が別画面でも行き止まりになりません。

### 独自版での判断

MVPは1つのInboxリスト、所要時間、subtasks、Activityへのschedule操作だけにします。複数リストとEisenhower分類は、Todoが増えてから追加します。

## Focus

### 対象Activity

Focus候補は次の条件で絞られます。

- 終日Activityを除外
- 完了済みを除外
- `Scheduled`、または開始済みの`Play`を対象
- 今日開始するもの、または開始済みで終了前のものを対象

### 確認したUI

- 1件を大きなcardで表示
- 複数候補は横pager
- icon、色、残り時間、checklist progress
- 開始前・進行中・一時停止を反映したcountdown
- 円形progressを直接dragして進捗更新
- 10%刻みのhaptic、100%時の成功feedback
- 完了後はFocus候補から除外

### ADHD向けの意味

Planの情報量をそのまま見せず、「今の1件」へ変換しています。時間の経過を数字だけでなく円形progressで可視化し、checklistによって開始点を小さくします。

### 独自版での判断

MVPではdrag可能なprogressを省きます。残り時間、pause、complete、subtaskの4操作に限定します。操作データが得られた後に手動progressを追加します。

## AI checklist

### 確認した挙動

- titleとdescriptionからchecklist候補を取得
- 自動分解機能には無料回数を表すfeature flagがある
- title/tagからemoji・tag候補を生成する別AI境界がある

### 独自版での判断

AIは主要データモデルではなく入力支援です。MVPはrule-based templateで開始します。

例:

- 「メールする」→ 宛先確認、要点3つ、送信、返信待ち記録
- 「外出する」→ 持ち物、移動、到着時刻
- 「資料を作る」→ 目的、見出し、初稿、見直し

実利用が確認できてから、明示的な「分解を提案」ボタンでLLMを呼びます。自動送信や勝手な書き換えは行いません。

## Onboarding

### 確認した流れ

1. サーバー配信の質問セットへ回答
2. 朝・昼・夜のroutine候補を選択
3. 選択routineを初期Activityとして作成
4. アカウント作成
5. 通知許可
6. testimonial / paywall

### 独自版での判断

初回起動時の長い質問は離脱要因になります。MVPでは次の3画面だけにします。

1. 何を助けてほしいか: 忘れ防止 / 時間の見える化 / 今やること
2. 今日の最初のTodoを1つ作る
3. 必要なら通知を許可する

routine templateはホームから後で追加できます。課金画面は初回価値を体験する前に出しません。

## Settingsと感覚調整

確認した設定:

- profile切り替え・編集
- locale、週開始日、12/24時間
- theme
- notification
- sounds
- haptic feedback
- product updates opt-in

ADHD支援では音・振動が助けにも刺激過多にもなります。独自版ではすべて個別に無効化でき、色だけに意味を依存させません。

## 独自版のUX原則

1. **次の1操作を常に見せる**: 空画面にも作成ボタンを1つだけ置く
2. **曖昧さを許す**: 日時未定のTodo、粗い時間帯を正式な状態として扱う
3. **失敗を罰しない**: streak喪失や赤い遅延警告を中心にしない
4. **完了を見える化する**: 小さなhaptic/animationは選択可能にする
5. **入力を段階化する**: titleだけで保存でき、詳細は後から足せる

## MVP受け入れ基準

- 10秒以内にTodoを1件追加できる
- Todoを3操作以内で今日のActivityへ変換できる
- アプリ起動後、2操作以内でFocusを開始できる
- Focus画面には同時に1件だけ表示される
- 通知、音、hapticをそれぞれ独立して無効化できる
- overdueでも責める文言を出さず、再計画を提案する
