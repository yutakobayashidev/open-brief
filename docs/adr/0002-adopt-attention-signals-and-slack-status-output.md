# ADR 0002: Attention SignalとSlack Status Outputを採用する

## Status

Accepted — 2026-07-21

## Decision

OpenBriefの注意遷移へ`Signal`段階を追加し、Slack custom statusを最初のOutput Adapterとして採用します。

Signalは、集中や探索に入る本人の状態を推測して自動公開する機能ではありません。本人が開始または延長を選んだとき、公開内容と終了時刻を確認できる形で、周囲へ応答状態を伝える機能です。

```text
Observe → Protect → Signal → Explore / Focus → Return
```

## Decision Drivers

- 集中や探索へ入ると、本人だけでなく会話相手にも応答不能の影響が生じる
- 「無視している」のか「いつ戻る」のかが分からない状態を減らしたい
- OpenBriefを情報のInputだけで完結させず、本人が選んだ注意状態を社会的contextへOutputしたい
- 自動返信より小さく、可逆で、誤動作時の損失が限定された出力から始めたい
- 作業内容や診断情報を公開せず、必要最小限のavailabilityだけを伝えたい

## Context

これまでのOpenBriefは、GmailやRSSを収集し、本人のためにProtect、Explore、Capture、Returnを支援するInput中心の構想でした。

しかし、集中や探索中にSlackの会話から離脱すると、次の問題は本人の画面内だけでは解決しません。

- 会話相手が返信時刻を予測できない
- 追加のpingや確認が増える
- 本人が後から未返信への焦りや罪悪感を持つ
- 集中を守るための離脱が、関係上は無言の不在に見える

Signalは、本人の集中を周囲へ説明し、復帰予定を共有するための出力境界です。

### Facts

- Slackの`users.profile.set`はcustom statusのtext、emoji、expirationを設定できる
- custom statusの更新にはuser tokenと`users.profile:write` scopeが必要である
- custom statusはavailabilityの表示であり、通知を止めるDNDとは別機能である
- ADR 0001は外部serviceへのwriteに明示確認境界を要求している

### Assumptions

- 初期利用者は、集中中の状態と復帰予定をSlackへ共有する価値を感じる
- genericなstatusでも、無言の離脱より会話相手の予測可能性が上がる
- 毎回別dialogを出さず、Focus開始画面内のpreviewと開始操作で十分な同意を得られる
- status更新に失敗しても、OpenBrief内のFocusやReturnは継続できる

## Signal Model

```text
ProtectedIntent / ReturnAnchor
              ↓
    AttentionSignalProposal
              ↓ preview + user action
       SlackStatusAdapter
              ↓
      AttentionSignalReceipt
              ↓ expire / reconcile
            Return
```

最小entityは次の責務を持ちます。

| Entity | Responsibility |
|---|---|
| `AttentionSignalProposal` | 公開先、status text、emoji、終了時刻の候補 |
| `AttentionSignalConsent` | workspace単位の許可と、今回の開始操作 |
| `AttentionSignalReceipt` | Slackへ実際に適用した値、時刻、結果 |
| `PreviousStatusSnapshot` | 上書き前のstatusと、復元可能性の判断材料 |

実装時の概念schemaは次を起点にします。

```ts
type AttentionSignalProposal = {
  adapter: 'slack_status'
  workspaceId: string
  statusText: string
  statusEmoji: string
  expiresAt: string
  includesWorkContext: boolean
}

type AttentionSignalReceipt = {
  proposalId: string
  appliedAt?: string
  appliedText?: string
  appliedEmoji?: string
  expiresAt?: string
  outcome: 'applied' | 'failed' | 'skipped'
  errorCode?: string
}
```

この型は実装前の固定APIではありません。ただし、proposalと実際の適用結果を分け、失敗を成功として扱わない境界は固定します。

## User Flow

### FocusまたはExplore開始

開始画面に、Slackへ公開するstatusと終了時刻を表示します。

```text
🧠 集中中・15:00ごろ戻ります
緊急の場合はメンションしてください

公開先: Example Workspace
終了: 15:00

[Slackで共有して開始] [共有せず開始]
```

`共有せず開始`を常に選べます。SignalはFocusやExploreの入場条件ではありません。

workspace設定で継続的な同期を許可していても、background inferenceだけを理由にstatusを更新しません。本人による開始または延長操作をtriggerにします。

### Focus延長

Focusを延長した場合だけ、新しいexpirationをpreviewして更新します。timerの遅れやmodel推定だけで延長しません。

### Return

終了時はSlack側のexpirationをcleanupの第一手段にします。OpenBriefが稼働している場合は現在statusを読み、OpenBriefが適用した値と一致するときだけclearまたは以前のstatus復元を試みます。

Slack上で本人がstatusを変更していた場合、その値を上書きしません。

## Privacy-safe Defaults

既定templateは、作業内容ではなくavailabilityだけを伝えます。

- 公開する: 集中中であること、復帰予定時刻、緊急時の連絡方法
- 公開しない: task title、email subject、project名、診断名、AIの推定状態
- permanent statusを作らず、expirationを必須にする
- AIによる自由文生成を必須にしない
- workspaceごとに個別に許可し、全workspaceへ自動fan-outしない

詳細な作業内容を含むtemplateは本人が明示的に選んだ場合だけ許容します。

## Ownership and Reconciliation

OpenBriefがSlack上の手動操作と競合しないため、次の順序を使います。

1. 書き込み前に現在のstatusを取得する
2. 置き換える内容とexpirationを本人へ表示する
3. 適用した完全な値をreceiptへ保存する
4. clearまたは復元前に現在のstatusを再取得する
5. 現在値がreceiptと一致しない場合は何もしない

アプリ停止やnetwork failureでreconcileできない場合も、Slack側のexpirationによってstatusが残り続けない設計にします。

以前のstatus復元はbest effortです。復元できない場合に、本人の新しいstatusを上書きしてまで整合させません。

## Failure Behavior

- Slack書き込み失敗はFocusやExploreを止めない
- UIは`共有済み`と表示せず、失敗と再試行操作を示す
- retryは同じproposalを使い、expirationが過去なら再確認する
- remote serviceの失敗を別workspaceへの送信で補わない
- status更新を繰り返しpollingせず、状態遷移時だけ書き込む

## Output Adapter Boundary

Slack固有処理はOutput Adapterへ閉じ込めます。

```ts
interface AttentionSignalAdapter {
  preview(proposal: AttentionSignalProposal): Promise<SignalPreview>
  readCurrent(target: SignalTarget): Promise<ExternalSignalState>
  apply(proposal: AttentionSignalProposal): Promise<AttentionSignalReceipt>
  reconcile(receipt: AttentionSignalReceipt): Promise<ReconcileResult>
}
```

domain側はSlack API method、token形式、rate limitを知りません。将来Calendar、Teams、Discordなどへ出力する場合も、各adapterが同じproposal、receipt、ownership原則に従います。

## Scope

### MVPに含める

- 1つのSlack workspaceへのcustom status
- privacy-safeな固定template
- status text、emoji、expirationのpreview
- Focus開始、延長、Returnに連動した更新
- 書き込み結果とownership check

### MVPに含めない

- Slack message、DM、mentionのInput収集
- DNDの自動設定
- channelへの自動投稿
- 自動返信または返信時刻の個別送信
- AIがハイパーフォーカスを推定して開始するstatus更新

Slack Input Adapterは、Gmail＋RSSの中核仮説を検証した後に扱います。Slack Status Output Adapterとは権限とriskが異なるため、同じ機能として実装しません。

## Options Considered

### A. OpenBrief内だけにFocus状態を表示する

採用しません。本人の注意は支援できますが、会話相手から見た無言の離脱は変わりません。

### B. Focus開始時に自動返信またはchannel投稿する

採用しません。通知量が増え、宛先や文脈を誤る損失がcustom statusより大きいためです。

### C. 行動を検知して完全自動でstatusを更新する

採用しません。誤推定、privacy disclosure、本人のagency低下を招きます。

### D. Slack custom statusを本人操作とexpiration付きで同期する

採用します。workspace全体へ低い通知負荷でavailabilityを示せて、失効による安全弁を持てます。

## Consequences

### Positive

- OpenBriefがInput整理だけでなく、社会的な注意調整を扱える
- 無言の離脱を、復帰予定のある状態へ変換できる
- 自動返信より通知負荷と誤送信riskが小さい
- Slack固有実装をOutput Adapterとして交換可能にできる
- Returnを個人作業だけでなく、会話への復帰として拡張できる

### Negative

- user token、write scope、workspace承認が必要になる
- statusが本人の実際の状態とずれる可能性がある
- 自動statusを監視や生産性表示と解釈される可能性がある
- workspaceごとの文化やstatus利用習慣に効果が依存する
- custom statusだけでは未読会話の復帰支援を提供できない

### Follow-up

- Signalを含むgolden caseとfixtureを別ケースとして作る
- Slack status adapterのcontract testを作る
- manual status変更を模したownership testを作る
- Signalが会話相手と本人へ与える効果をpilotで測る
- Slack Inputとre-entry briefは別の判断として設計する

## Research Hypothesis

Signalの効果は未検証です。最初の仮説を次のように置きます。

> 本人操作とexpirationを伴うSlack statusは、privacy discomfortを増やさず、集中中の応答可能性を会話相手が予測しやすくする。

主要な観測候補は次です。

- Focus中の追加ping数
- Focus終了前の応答期待に関する問い合わせ数
- 会話相手が認識した復帰予定の正確さ
- 本人の未返信不安とReturn後の会話復帰時間
- status内容に対するprivacy discomfort

message数の減少だけを成功にしません。緊急連絡を遅らせたり、会話相手がstatusを信用しなくなった場合は失敗です。

## Adoption and Exceptions

code reviewとtestで次を要求します。

- expirationのないSlack statusを書き込まない
- explicitな開始または延長操作なしにstatusを書き込まない
- task titleなどのwork contextをdefault templateへ含めない
- manual変更後のstatusをclearまたは復元で上書きしない
- API失敗時にFocusを止めず、共有成功とも表示しない

例外はrepository maintainerが承認します。例外には、公開data、trigger、expiration、manual override、失敗時動作を示すtestとUI説明が必要です。

恒久的な変更は本ADRを曖昧に書き換えず、新しいADRで`Superseded`にします。

## Open Questions

- 最初のstatus有効時間と上限を何分にするか
- 以前のstatusを自動復元するか、候補だけ提示するか
- 複数workspace対応をいつ追加するか
- 緊急連絡方法を固定文にするか、workspaceごとに設定するか
- Slack会話へのReturnをどのeventで測るか

## References

- Slack, [`users.profile.set`](https://api.slack.com/methods/users.profile.set)
- Slack, [Web API methods](https://api.slack.com/web)
- [ADR 0001: Local-firstなデータ境界とModel Gateway](0001-adopt-local-first-data-and-model-boundaries.md)
