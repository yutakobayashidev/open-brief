# Entire CLI source reference

## 固定判断

Entire CLIのGo packageやGit checkpoint storeをOpenBriefへ組み込まない。provider固有hookをnormalized eventへ変換する境界、pure lifecycle、atomic state update、human / JSON CLI、bounded external protocolのpatternをRustで小さく再実装する。

Entireは通常時にdaemonを持たないため、ambient window / screen collectorのprocess modelには使えない。Agent hookはtimelineへ加える一sourceとして扱い、OpenBrief全体のsession開始条件にはしない。

## 調査基準

| 項目 | 値 |
|---|---|
| Repository | [entireio/cli](https://github.com/entireio/cli) |
| 基準release | [`v0.9.0`](https://github.com/entireio/cli/tree/8b77ad43132d18f7958825c9dcd26544ab8f5d92) |
| 固定SHA | `8b77ad43132d18f7958825c9dcd26544ab8f5d92` |
| 調査時main | `683a10d3773ee4830ee791f39d76d8092ef1b0cf` |
| License | [MIT](https://github.com/entireio/cli/blob/8b77ad43132d18f7958825c9dcd26544ab8f5d92/LICENSE) |
| 調査日 | 2026-07-30 |

codeを移植する場合はMIT copyright / license noticeを保持する。ただしrepositoryは大きく、OpenBriefはRustで実装するため、module単位の直接利用より設計patternの参照が適する。

## 実行モデル

Entireの通常flowは短命processの集合である。

```text
AI agent hook
  → entire hooks <agent> <event>

Git hook
  → prepare-commit-msg / post-commit等

process間state
  → .git/entire-sessions/*.json

external agent
  → stateless subcommand + stdin/stdout
```

`entire mcp`だけはhostが起動している間のstdio JSON-RPC serverである。window eventを常時subscribeするdaemonやUnix socket ownerではない。

したがってOpenBriefは次のprocess modelを変えない。

```text
openbrief watch
  長寿命collector / single writer

openbrief status / today / around / pause / resume
  collectorのcontrol/query client

agent hook
  optional event producer
```

## Source map

| Source | 責務 |
|---|---|
| [`cmd/entire/main.go`](https://github.com/entireio/cli/blob/8b77ad43132d18f7958825c9dcd26544ab8f5d92/cmd/entire/main.go) | signal、exit、plugin dispatch |
| [`cli/root.go`](https://github.com/entireio/cli/blob/8b77ad43132d18f7958825c9dcd26544ab8f5d92/cmd/entire/cli/root.go) | Cobra command tree |
| [`cli/lifecycle.go`](https://github.com/entireio/cli/blob/8b77ad43132d18f7958825c9dcd26544ab8f5d92/cmd/entire/cli/lifecycle.go) | normalized event dispatcher |
| [`agent/agent.go`](https://github.com/entireio/cli/blob/8b77ad43132d18f7958825c9dcd26544ab8f5d92/cmd/entire/cli/agent/agent.go) | provider interfaceとoptional capability |
| [`agent/event.go`](https://github.com/entireio/cli/blob/8b77ad43132d18f7958825c9dcd26544ab8f5d92/cmd/entire/cli/agent/event.go) | normalized lifecycle event |
| [`agent/registry.go`](https://github.com/entireio/cli/blob/8b77ad43132d18f7958825c9dcd26544ab8f5d92/cmd/entire/cli/agent/registry.go) | provider factory registry |
| [`session/phase.go`](https://github.com/entireio/cli/blob/8b77ad43132d18f7958825c9dcd26544ab8f5d92/cmd/entire/cli/session/phase.go) | pure lifecycle transition |
| [`session/state.go`](https://github.com/entireio/cli/blob/8b77ad43132d18f7958825c9dcd26544ab8f5d92/cmd/entire/cli/session/state.go) | active state JSON |
| [`checkpoint`](https://github.com/entireio/cli/tree/8b77ad43132d18f7958825c9dcd26544ab8f5d92/cmd/entire/cli/checkpoint) | ephemeral / persistent Git store |
| [`transcript`](https://github.com/entireio/cli/tree/8b77ad43132d18f7958825c9dcd26544ab8f5d92/cmd/entire/cli/transcript) | native transcriptとderived compact view |
| [`redact`](https://github.com/entireio/cli/tree/8b77ad43132d18f7958825c9dcd26544ab8f5d92/redact) | secret / PII redaction |

## 採るpattern

### Provider adapterからnormalized eventへ

各agentはnative hook payloadを[`Event`](https://github.com/entireio/cli/blob/8b77ad43132d18f7958825c9dcd26544ab8f5d92/cmd/entire/cli/agent/event.go#L83)へ変換し、その後のorchestrationをproviderから分離する。

```text
native hook
  → AgentHookAdapter
  → normalized event
  → central dispatcher
  → lifecycle / store
```

OpenBriefではAgent eventをambient timelineと同じenvelopeへ入れる。

```text
WindowActivated
IdleStarted / IdleEnded
ScreenSampleCaptured
AgentTurnStarted / AgentTurnEnded
PolicyDenied
RecordingPaused / RecordingResumed
ReflectionGenerated
```

Agent eventは意味の強い補助signalだが、goalやsessionがない作業を失わないため、timelineの親にはしない。

### Pure transition、effectは外

Entireのsession phaseは`active / idle / ended`をpure transitionで扱い、side effectをactionとして返す。

OpenBriefもcollector state、capture reservation、privacy epochを同じ形にする。

```text
State + Event
  → NewState + Action[]

Action executor
  → capture / cancel / commit / gap
```

これにより、同じevent列からrace、restart、late resultをdeterministicにtestできる。

### Validation choke point

[`DispatchLifecycleEvent`](https://github.com/entireio/cli/blob/8b77ad43132d18f7958825c9dcd26544ab8f5d92/cmd/entire/cli/lifecycle.go#L49)は、filesystem pathへ届くsession / tool / subagent IDをhandler分岐前に検証する。

OpenBriefもsource adapterごとにvalidationを複製せず、normalized event受理地点で次を検証する。

- ID、timestamp、path
- app / window identity
- payload byte上限
- schema version
- capabilityとprivacy epoch

### Atomic state update

Entireのactive state保存は次の順序である。

1. session IDを検証
2. unique temporary fileへwrite
3. close
4. directory-scoped rename

read-modify-write全体はper-session OS lockで囲む。atomic renameだけではlost updateを防げない。

OpenBriefのtimeline本体はsingle-writer SQLite / append storeを優先する。JSON side-stateやjob spoolを置く場合だけ、このpatternを採る。

### Hook inputをEOF待ちしない

[`ReadHookInputRawLimited`](https://github.com/entireio/cli/blob/8b77ad43132d18f7958825c9dcd26544ab8f5d92/cmd/entire/cli/agent/event.go#L199)は`io.ReadAll`ではなくstreaming JSON decoderで最初の完全なvalueを読む。writerがpipeを閉じない環境でもhookが永久停止しない。

OpenBriefのhook adapterも次を要求する。

- one JSON value
- byte上限
- deadline
- interactive stdinなら即error
- stdoutはresponseだけ、diagnosticはstderr

### Failure policyを分ける

Entireの観測hookは多くの場合Git操作を止めない。一方、明示的に有効化されたprivacy filterが失敗したpushは停止する。

OpenBriefも次へ分ける。

```text
observation failure
  → 作業を止めず、理由付きgap

privacy / export failure
  → capture、response、commit、egressをdeny
```

### Canonical sourceとderived view

Entireはagent native transcriptをauthorityとして保存し、compact transcriptを別生成する。

OpenBriefではraw agent transcriptを複製しないが、原則は採る。

```text
canonical:
  timestamped ActivityEvent

derived:
  FocusSegment
  ActivityObservation
  ActivitySlice
  Reflection
```

derived dataは削除・再生成可能にし、authorityと混ぜない。

### CLI UX

参考にする点:

- human outputと`--json`を分ける
- primary dataはstdout、notice / diagnosticはstderr
- accessible modeではinteractive TUIを避ける
- hidden infrastructure commandとuser commandを分離
- command treeからagent向けhelpを生成する
- first signalでcancel、二回目でforce exit

OpenBriefはRust `clap`で原則だけ採り、Entireのcommand数を再現しない。

## External provider protocol

Entireは`entire-agent-<name>`を`$PATH`から見つけ、`info`でprotocol versionとcapabilityを得る。

主なboundary:

- stateless subcommand
- JSON stdin / stdout
- protocol version
- optional capability declaration
- process timeout
- input / output size上限
- discovery timeout
- built-in providerとの名前衝突拒否

[protocol document](https://github.com/entireio/cli/blob/8b77ad43132d18f7958825c9dcd26544ab8f5d92/docs/architecture/external-agent-protocol.md)

これはproviderを別binaryとして第三者へ開放する段階で有用である。MVPではRust workspace内のsmall traitで十分。

## OpenBriefの小traitへの変換

```text
EventSource
  subscribe() -> ActivityEvent

AgentHookAdapter
  parse(native) -> ActivityEvent[]

EvidenceCapture
  capture(intent) -> RawEvidence

PolicyGate
  authorize(context, capability) -> Decision

EvidenceStore
  append(event)
  query(range, filter)

Reflector
  summarize(segment) -> Reflection
```

Entireの`Agent` interface全体はlegacy責務も含むため再現しない。

## 採用しない範囲

| 対象 | 理由 |
|---|---|
| Go package直接依存 | Rust projectとprocess modelが異なる |
| daemonless hook model | ambient window / screen観測ができない |
| Git shadow branch / checkpoint refs | desktop timelineと寿命・queryが異なる |
| checkpointごとのfull transcript | capacityとprivacy costが高い |
| 巨大`Agent` interface | identity、storage、resume等が混在 |
| cloud auth / control plane | local-first MVPに不要 |
| MCP server | query contractが固まる前には不要 |
| redaction package移植 | typed privacy boundaryのpatternだけでよい |

## 再調査する条件

次のどれかが発生するまでは、stable `8b77ad4…`を基準にしてrepository全体を再調査しない。

1. Codex / Claude / Gemini hook adapterを実装する
2. 複数hook processでlost updateが発生する
3. external provider SDKを公開する
4. MCP経由でtimeline queryを提供する
5. transcript resume / importを実装する
6. Git commitとactivity evidenceを関連付ける
7. worktreeをまたぐAgent session統合が必要になる
8. cloud export前のredaction policyを実装する
