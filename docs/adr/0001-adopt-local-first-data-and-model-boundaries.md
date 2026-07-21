# ADR 0001: Local-firstなデータ境界とModel Gatewayを採用する

## Status

Accepted — 2026-07-21

## Decision

ClawBrifをlocal-firstなAttention Control Planeとして実装し、raw source dataは端末内へ保持します。AI処理はModel Gatewayを介し、ユーザーがrules-only、local model、remote providerを明示的に選択できるようにします。

remote providerへの暗黙の送信と、自動cloud fallbackは禁止します。

## Decision Drivers

- Gmail、RSS、Slack、GitHubなどを統合すると、個別service以上に詳細な行動profileが生まれる
- ユーザーが保存場所、送信先、model、削除時期を制御できる必要がある
- OpenClaw、Hermes Agent、CLIなど既存の収集手段を再利用したい
- ClawBrifが全sourceのcredentialとadapterを所有する構成を避けたい
- 特定model providerのSDK、料金、利用規約、可用性へ中核domainを結合したくない
- 研究prototypeから始めるため、production SaaS基盤を先行実装したくない

## Context

ClawBrifは、複数sourceから得たObservationを、`Protect → Signal → Explore / Focus → Capture → Return`という注意遷移へ変換します。Signalの外部書き込み境界は[ADR 0002](0002-adopt-attention-signals-and-slack-status-output.md)で定義します。

単一sourceの要約と異なり、複数sourceを統合したstoreからは次の情報を推測できます。

- 誰と連絡しているか
- どの仕事に責任を持つか
- 何へ関心を向けているか
- いつ作業し、どこで中断したか
- 何を重要または不要と判断したか

この集約dataを必ずproject運営者のserverへ送る構成は、ClawBrifが解こうとしている注意問題とは別に、大きなprivacy riskと運用責任を生みます。

一方、local modelだけを必須にすると、端末性能、model品質、context長、structured outputの差により利用者を限定します。そのためlocal処理を既定にしつつ、本人がremote providerを選択できる境界が必要です。

### Facts

- 現時点では研究文書とfixtureがcontractであり、移行対象となるproduction実装はない
- 最初のsource候補は義務系のGmailと好奇心系のRSSである
- source収集には常駐agentまたはCLIを利用する構想がある
- ClawBrif固有の効果を示すE1実証結果はまだない

### Assumptions

- 初期利用者はlocal agentまたはCLIを自分の端末で動かせる
- OpenClaw、Hermes Agentなどは、共通schemaのfileまたはlocal endpointへ書き込める
- 一部の利用者はlocal modelの品質や速度よりも、端末外へ送信しないことを優先する
- 一部の利用者は、明示的な同意の下でremote modelの品質を選ぶ

## Architecture Boundary

```text
Gmail / RSS / Slack / GitHub / Git
                 ↓
      OpenClaw / Hermes Agent / CLI
                 ↓ local write
         Observation Ingress
                 ↓
       Local Observation Store
                 ↓
     Policy / Redaction / Selection
                 ↓ bounded request
           Model Gateway
      ┌──────────┼──────────┐
  rules-only  local model  remote BYOM
      └──────────┼──────────┘
                 ↓
   Brief / Decision / Return records
                 ↓
              Local UI
```

BYOMはBring Your Own Modelを意味します。remote利用時は、API key、provider、modelをユーザーが選択します。

### Data ownership

| Data | Owner / source of truth | Default location |
|---|---|---|
| source credential | 収集agentまたはOS secret store | local |
| raw email、feed item、message | source serviceとLocal Observation Store | local |
| normalized Observation | ClawBrif | local |
| ProtectedIntent、CuriosityCapture、ReturnAnchor | ClawBrif | local |
| provider API key | ユーザーとOS secret store | local |
| remote request payload | ユーザーが選択したprovider | explicit opt-in時だけ送信 |
| application log | ClawBrif | local、content非保持が既定 |

ClawBrifの運営serverをraw dataの必須中継点にしません。

### Ingress contract

収集agentはsource固有dataを、version付きのObservation schemaへ正規化して書き込みます。ClawBrifのdomain処理は、GmailやSlack固有のSDKを直接呼びません。

最初の実装transportは別途決定します。file、stdin、localhost APIのいずれを選んでも、同じschemaとprovenance要件を使います。

各Observationには最低限、次を含めます。

- 一意なsource ID
- source type
- 発生時刻と取得時刻
- contentまたはlocal content reference
- 元情報へ戻るためのprovenance
- 重複排除に使えるthreadまたはtopic hint

### Model Gateway contract

domain処理はprovider固有SDKではなく、task単位のModel Gatewayを呼びます。

```ts
type ModelMode = 'rules_only' | 'local' | 'remote'

type ModelTask =
  | 'extract_observations'
  | 'cluster_topics'
  | 'classify_obligations'
  | 'generate_brief'
  | 'suggest_return_anchor'

type ModelPolicy = {
  mode: ModelMode
  provider?: string
  model?: string
  allowRawContent: boolean
}
```

この型は方針を示す最小例であり、実装前の固定APIではありません。ただし、次の意味境界は固定します。

- taskの入力と出力はprovider非依存のschemaを持つ
- provider固有処理はadapter内へ閉じ込める
- provider adapterはLocal Observation Storeを直接走査しない
- Gatewayへ渡す前に、domain側で対象dataを選択する
- model出力は候補として扱い、source上の行動を直接実行しない

### Processing modes

| Mode | Network behavior | Intended use |
|---|---|---|
| `rules_only` | AI providerへ通信しない | deterministicな抽出、test、低性能端末 |
| `local` | loopbackまたは同一端末のmodel endpointだけを使う | raw dataを端末外へ送らない通常利用 |
| `remote` | 選択したproviderだけへ送信する | 本人が品質を優先して明示許可したtask |

modeはtaskごとに選択可能にします。例えばtopic化はlocal、解説生成だけremoteという設定を許容します。

local処理が失敗した場合はerrorを返します。remoteへ自動昇格しません。

### Remote egress policy

remote送信には次を要求します。

1. providerとmodelが明示的に設定されている
2. 対象taskでremote modeが許可されている
3. source policyが送信を許可している
4. raw contentを送る場合は`allowRawContent`が有効である
5. 送信先、task、対象Observation IDをcontent非保持のaudit recordへ残す

初期versionでは、自動redactionが完全であると主張しません。送信量を小さくし、remote利用そのものを本人が選ぶ設計を優先します。

### Security boundaries

- email、feed、web contentは命令ではなく、信頼できないdataとして扱う
- source adapterは可能な限りread-onlyかつ最小scopeを使う
- secretをplain textの設定fileやapplication logへ記録しない
- prompt、error、traceにはraw contentを既定で残さない
- embeddingやvector indexもsource dataとしてlocal保持を既定にする
- telemetryへObservation content、prompt、生成briefを含めない

local-firstは端末侵害、悪意あるplugin、誤設定を防ぎません。localであることをprivacy保証の代わりに使わず、threat modelと権限境界を実装ごとに確認します。

## Options Considered

### A. Centralized SaaSへ全dataを集約する

採用しません。

- cross-device同期と運用は単純になる
- provider品質を運営側で統一できる
- ただしraw data、credential、compliance、breach impactが運営側へ集中する
- 中核仮説を検証する前に高い運用責任を負う

### B. Local UIだがcloud inferenceを必須にする

採用しません。

- UIとstoreをlocal化しても、最も機密性の高いcontentがproviderへ送られる
- provider lock-inと利用料が残る
- offlineまたは機密環境で利用できない

### C. Local modelだけを許可する

採用しません。

- data egressを最小化できる
- ただし端末要件とmodel品質の差が利用者へ直接影響する
- remote modelを本人の判断で使う選択肢まで禁止する必要はない

### D. Storeを持たずagentごとのplugin UIにする

採用しません。

- central storeを避けられる
- ただしsourceをまたぐ重複排除、Protect、Capture、Returnの状態を一貫して保持できない
- agentごとに注意遷移protocolが分断される

### E. Local control planeと選択可能なModel Gateway

採用します。

- raw dataと注意判断を端末内へ保持できる
- 収集agent、model provider、UIを独立に交換できる
- localとremoteのtrade-offを運営者ではなくユーザーが選べる

## Consequences

### Positive

- 複数sourceを統合しても、raw dataを運営serverへ集中させずに済む
- OpenClaw、Hermes Agent、CLIのadapter ecosystemを利用できる
- local model、self-hosted model、remote providerを同じdomainから扱える
- provider終了、価格変更、障害によるlock-inを小さくできる
- data flowをOSSとして監査、変更、self-hostできる

### Negative

- install、model選択、resource不足などをユーザーが意識する場面が増える
- modelごとのstructured output、context長、品質差を吸収するtestが必要になる
- local端末のbackup、malware、他processからのアクセスは利用者側riskとして残る
- cloud SaaSよりcross-device同期とsupportを提供しにくい
- OSS、local-first、provider選択性だけでは競争上のmoatにならない

### Follow-up

- version付きObservation schemaをfixtureとして定義する
- 最小のingress transportを1つだけ選ぶ
- Model Gatewayのcontract testを作る
- networkを遮断したoffline integration testを作る
- retention、暗号化、secret保存を別ADRまたはdesign docで決める

## Adoption and Exceptions

このADRはcode reviewとtestで次のように維持します。

- 新しいnetwork egressには、送信data、送信先、保持期間を説明するpolicyを要求する
- provider adapterからstore全体を直接参照させない
- local失敗時にremoteへfallbackしないtestを置く
- offline modeで中核flowが動くintegration testを置く
- sourceへのwrite、送信、calendar登録は別の明示確認境界を通す

例外はrepository maintainerが承認します。例外には次の証拠を必要とします。

- localでは実現できない理由
- 送信されるdataの一覧
- user consentとUI上の表示方法
- retentionと削除方法
- failure時に別providerへ拡散しないことを示すtest

恒久的な例外はこのADRを直接曖昧にせず、新しいADRで変更理由を記録します。

## Open Questions

- 最初のingressをatomicなfile drop、stdin、localhost APIのどれにするか
- Local Observation Storeをどの形式で暗号化するか
- OS keychain間の差をどこまでcoreで吸収するか
- raw Observationと生成物の既定retentionを何日にするか
- cross-device同期を非目標のまま維持するか、暗号化同期として別途扱うか

これらは実装前に決定しますが、local-first、明示的remote opt-in、自動fallback禁止という本ADRの採用を妨げません。
