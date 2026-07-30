# Buzz source reference

## 固定判断

Buzz全体をforkせず、workspace crateへも依存しない。Tauri / Rust側でAgent processを所有する境界、ACP subprocessとのbounded JSON-RPC、runtime catalog、readiness、generation付きlifecycleのpatternをOpenBrief向けに小さく再実装する。

Buzzの中心はNostr relayをauthorityとするteam workspaceであり、OpenBriefのlocal Brief Storeとはdomainもdeploymentも異なる。Nostr、Postgres、Redis、MinIO、agent pool、team identity、workflow engineは持ち込まない。

ACPはDesktopとstateful Agentを結ぶcontrol planeとして使う。Hermes cron等からのObservation投入、OpenBriefのAgent tool、LM Studio等のLLM Providerは、それぞれingress、MCP、Model Gatewayとして分離する。

## 調査基準とlicense

| 項目 | 値 |
|---|---|
| Repository | [block/buzz](https://github.com/block/buzz) |
| 固定SHA | [`63496cc1d4c6f1b7c613801bdcc694169dcf391a`](https://github.com/block/buzz/tree/63496cc1d4c6f1b7c613801bdcc694169dcf391a) |
| Commit date | 2026-07-29 |
| License | [Apache License 2.0](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/LICENSE) |
| 調査日 | 2026-07-30 |

codeを移植する場合はApache-2.0のcopyright、license、NOTICE要件を確認し、変更したfileへ変更の明示を残す。現時点では直接移植せず、source pathとpatternだけを参照する。

BuzzのACP clientはBuzz固有のrelay、Codex環境変数merge、Goose / Claude extension、usage accountingまで含む。OpenBriefでは公式ACP Rust libraryまたは最小clientを先に評価し、`acp.rs`全体をコピーしない。

## Buzzが実際に所有するもの

Buzzは人間とAgentが同じroomへ参加するself-hosted team workspaceである。全message、reaction、workflow、Git eventを署名付きNostr eventとしてrelayへ集約する。

```text
Buzz Desktop / CLI / Agent
             │
             ▼
        Buzz Relay
      Nostr event authority
       ├─ Postgres
       ├─ Redis
       └─ S3 / MinIO
```

このauthorityはOpenBriefへ採らない。

```text
OpenBrief
  local Observation / Brief / UserDecision Store
```

OpenBriefのBriefはteam chat eventではなく、本人の情報源から作られた有限なsnapshotと判断履歴である。

## Agent連携の実行モデル

Buzzのagent pathはDesktopからAgentへ直接ACP requestを送る一段構成ではない。

```text
Buzz Desktop
  └─ buzz-acp processを起動・監視

Buzz Relay
  └─ @mention event
       ↓ WebSocket
     buzz-acp
       ↓ stdio ACP / JSON-RPC
     Codex / Claude / Hermes / OpenClaw等
       ↓ Buzz CLI
     Buzz Relayへ返信
```

[`buzz-acp`](https://github.com/block/buzz/tree/63496cc1d4c6f1b7c613801bdcc694169dcf391a/crates/buzz-acp)は次を担当する。

- relay接続とchannel discovery
- channelごとのevent queue
- Agent process pool
- ACP session作成とprompt
- response中のtool / progress処理
- crash時のrespawn
- Agentが利用するBuzz MCP / CLI設定

[`ARCHITECTURE.md`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/ARCHITECTURE.md#buzz-acp--agent-communication-protocol-harness)では、`buzz-acp`自身はstateを永続化しない。relayがauthorityであり、harnessはeventとAgent sessionを接続するruntimeである。

OpenBriefのDesktop MVPにはrelayやprocess poolがないため、次へ単純化する。

```text
OpenBrief Tauri core
  └─ OpenBriefAgentClient
       └─ ACP Agent subprocess

OpenBrief local store
  └─ Brief / source reference / UserDecision authority
```

将来cron eventや複数consumerを調停する必要が出ても、最初からBuzz型harnessを別processにしない。Tauriを閉じても動く`openbriefd`が必要になった時だけ、Agent runtimeのownerをdaemonへ移す。

## Buzz CLI、ACP、MCPの関係

`buzz` CLI自体はACPを話さない。ACPはAgentへのpromptとsession lifecycleを運び、CLIはAgentがBuzz Relayを読み書きするaction planeである。

```text
Human / Buzz Desktop
        ↓ Nostr message
     Buzz Relay
        ↓ WebSocket
     buzz-acp
        ↓ ACP / stdio JSON-RPC
     ACP Agent
        ↓ own shell、またはbuzz-dev-mcpのshell
      buzz CLI
        ↓ HTTP + 署名付きNostr event
     Buzz Relay
```

固定snapshotの往復は次の通り。

1. Desktopが`buzz-acp`へ`BUZZ_PRIVATE_KEY`、`BUZZ_RELAY_URL`、Agent command、optional MCP commandを渡して起動する
2. `buzz-acp`がRelay eventをchannelごとにqueueし、contextとreply先を付けて`session/prompt`する
3. Agentが`buzz messages get / thread / send`等をshell commandとして実行する
4. `buzz` CLIがAgent identityでeventへ署名し、Relayへpublishする
5. Relayが投稿をDesktopと他のparticipantへ配信する

[`base_prompt.md`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/crates/buzz-acp/src/base_prompt.md#L64-L74)は、価値のある結果を必ず`buzz messages send`でpublishするようAgentへ要求する。[`queue.rs`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/crates/buzz-acp/src/queue.rs#L1144-L1171)はtrigger eventに応じた`--reply-to`をpromptへ注入する。

重要なのは、ACPの[`agent_message_chunk`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/crates/buzz-acp/src/acp.rs#L1693-L1720)がBuzz messageへ自動変換されず、log / observerに使われるだけという点である。AgentがCLIを呼ばずにACP turnを終えると、channelには回答が出ない。複数投稿、reaction、DM、workflow等をAgent自身が選べる一方、prompt遵守に依存するsilent failureを持つ。

### `buzz-dev-mcp`はBuzz message専用MCPではない

`buzz-acp`はoptional MCP descriptorをACPの`session/new.mcpServers`へ渡す。固定snapshotではCodexとBuzz Agentへ`buzz-dev-mcp`を設定し、GooseとClaude Codeには設定しない。

[`buzz-dev-mcp`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/crates/buzz-dev-mcp/src/lib.rs#L30-L135)が提供するのはshell、file edit、image、todo等のgeneral development toolである。そのshell用PATHに`buzz` multicall shimを置き、`buzz`として起動された同一binaryが`buzz_cli::run_from_args`へdispatchする。

```text
ACP Agent
  → MCP shell
  → buzz shim
  → buzz-cli
  → Relay
```

GooseやClaude CodeのようにMCP sidecarを使わないruntimeは、Agent自身のshellからPATH上のbundled `buzz`を直接呼ぶ。したがってMCPはCLIのprotocol adapterではなく、CLIを実行できるtool surfaceの一つである。

### OpenBriefでは返信transportを分けない

個人用OpenBrief Desktopでは、ACPの`agent_message_chunk`をtyped eventとして直接UIへ表示する。Agentに`openbrief` CLIを実行させなければ会話へ回答できない構造は採らない。

`openbrief` CLI / MCPは、timelineとBriefのbounded read、reversible triage等のdomain toolに限定する。共有channelへの複数投稿や別Agentへのdelegationが必要になった場合だけ、Agentが投稿先を選ぶBuzz型action planeを再評価する。

## Source map

| Source | 責務 | OpenBrief判断 |
|---|---|---|
| [`ARCHITECTURE.md`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/ARCHITECTURE.md) | relay、crate、ACP harness全体 | domain境界の確認だけ |
| [`crates/buzz-cli/README.md`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/crates/buzz-cli/README.md) | Agent-first JSON CLI、Relay REST client | CLI contractだけ参考 |
| [`crates/buzz-acp/src/base_prompt.md`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/crates/buzz-acp/src/base_prompt.md) | CLI利用とpublish policy | silent failureを含めて比較 |
| [`crates/buzz-acp/src/acp.rs`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/crates/buzz-acp/src/acp.rs) | stdio ACP client、timeout、cancel、process cleanup | patternを採る |
| [`crates/buzz-acp/src/queue.rs`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/crates/buzz-acp/src/queue.rs) | per-channel queue、batch、dedup | multi-session負荷が出るまで採らない |
| [`crates/buzz-acp/src/pool.rs`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/crates/buzz-acp/src/pool.rs) | 1〜32 Agent process pool | MVPでは採らない |
| [`crates/buzz-dev-mcp/src/lib.rs`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/crates/buzz-dev-mcp/src/lib.rs) | shell、file、todo、CLI multicall | OpenBrief MCPへ丸ごと採らない |
| [`managed_agents/discovery.rs`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/desktop/src-tauri/src/managed_agents/discovery.rs) | runtime catalog、PATH、version、auth probe | 小さなcatalogへ変換 |
| [`managed_agents/readiness.rs`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/desktop/src-tauri/src/managed_agents/readiness.rs) | 設定、binary、auth readiness | capability probeを採る |
| [`managed_agents/runtime.rs`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/desktop/src-tauri/src/managed_agents/runtime.rs) | env、spawn、process group、receipt | lifecycle patternを採る |
| [`managed_agents/runtime_commands.rs`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/desktop/src-tauri/src/managed_agents/runtime_commands.rs) | Tauri start / stop / restart / status | Rust ownershipを採る |
| [`managed_agents/custom_harnesses.rs`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/desktop/src-tauri/src/managed_agents/custom_harnesses.rs) | user定義runtime JSON | 複数runtimeの後に評価 |
| [`app_state.rs`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/desktop/src-tauri/src/app_state.rs#L18-L50) | process map、transition lock | Tauri coreのownershipに採る |
| [`tauriManagedAgents.ts`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/desktop/src/shared/api/tauriManagedAgents.ts) | typed frontend invoke wrapper | frontend境界に採る |
| [`managed_agents/backend.rs`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/desktop/src-tauri/src/managed_agents/backend.rs) | bounded one-shot backend | health / discovery commandだけに参考 |

## ACP clientから採るpattern

### stdoutをprotocol専用にする

[`AcpClient`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/crates/buzz-acp/src/acp.rs#L1-L35)はAgent subprocessのstdin / stdoutをnewline-delimited JSON-RPCへ固定する。diagnosticはstderrへ分離する。

```text
stdin / stdout
  ACP JSON-RPC only

stderr
  diagnostic / Agent log
```

OpenBriefでもstdoutへbanner、progress text、debug dumpを混ぜない。webviewへ渡すeventはRust側でtyped eventへ変換する。

### protocolとcapabilityを交渉する

Buzzの基本flow:

```text
spawn
  → initialize
  → session/new
  → session/prompt
  ← session/update*
  ← prompt response + stopReason
  → session/cancel
```

OpenBriefはAgent名から機能を決め打ちしない。ただし、すべての機能が`initialize`だけで交渉されるわけではない。

- protocol versionと`agentCapabilities`は`initialize` responseで確認する
- MCP serverは`session/new` requestへ明示的に渡す
- permissionはAgentから届く`session/request_permission` requestをuser approval brokerへ渡す
- session load等は該当capabilityを確認してからrequestする

Buzzの[`initialize`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/crates/buzz-acp/src/acp.rs#L589-L608)は、upstream RFD前のACP v2を一時的に要求し、`_meta.steering`等のBuzz固有extensionも読む。OpenBriefはこの`protocolVersion = 2`やextension probingをコピーせず、実装時点の公式ACP schemaとlibraryを基準にする。

### permissionを自動承認しない

Buzzは[`session/request_permission`を`allow_once`で自動承認](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/crates/buzz-acp/src/acp.rs#L1856-L1938)する。team room内のautonomous harnessとしての判断であり、本人のlocal dataとactionを扱うOpenBriefには採らない。

OpenBriefではpermission requestをtyped eventとしてDesktopへ渡し、既定をdenyにする。read-only query、reversible local triage、external writeを別capabilityに分け、external writeは明示的なuser approvalなしに実行しない。

### inputをboundedにする

Buzzは[`MAX_LINE_SIZE = 10 MB`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/crates/buzz-acp/src/acp.rs#L19-L21)を[`LinesCodec`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/crates/buzz-acp/src/acp.rs#L537-L541)へ設定し、1行の上限を置く。

10 MBという値をそのまま採らず、OpenBriefのevent種別ごとに小さな上限を置く。

```text
agent text update
  bounded UTF-8

tool metadata
  bounded JSON
```

ACP image contentはbase64をline内へ含め得るため、OpenBrief MVPの対話sessionではimage inputを無効にする。screen captureは既存のModel Gatewayへbounded requestとして渡す。Agentが画像を読む必要が実証された時に、上限を含むACP image対応または解決方法を別途設計する。

### idle timeoutと絶対上限を分ける

Buzz harnessはagent stdout activityで更新するidle timeoutと、turn全体のabsolute durationを分ける。長いtool実行を単純なwall timeoutだけで誤って殺さず、無限streamにはabsolute capを持つ。

OpenBriefでは最初から複雑なadaptive timeoutを作らず、次の二つだけを設定する。

- 最後のprotocol activityからのidle timeout
- session turnのabsolute timeout

### cancel後のcleanupを完了させる

cancel notificationを送るだけではchild processやpending responseが残る。Buzzの[`cancel_with_cleanup`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/crates/buzz-acp/src/acp.rs#L908-L1025)はcancel後に対象responseをbounded drainし、timeoutをcallerへ返す。process groupの停止とwaitはその後のshutdown / respawn側の責務である。

OpenBriefも次を同じlifecycleへ含める。

```text
cancel request
  → bounded drain
  → child tree termination
  → wait
  → runtime state更新
```

`kill`だけ実行してwaitしないzombie pathを作らない。

## Tauri runtime管理から採るpattern

### Rust側をprocess ownerにする

Buzzは[`AppState.managed_agent_processes`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/desktop/src-tauri/src/app_state.rs#L43-L50)へchild process mapを保持し、frontendはTauri commandでstart / stop / restart / listを呼ぶ。

OpenBriefもReactからshell stringを組み立てたりPIDをauthorityにしない。

```text
React
  typed AgentRuntimeId
       ↓ invoke
Tauri Rust
  allowlisted command / args
  child handle
  secrets
  lifecycle
```

frontendへAPI key、Bearer token、full environment、raw stderrを渡さない。

### lifecycle transitionを直列化する

Buzzはspawn / register、adopt、stop、shutdown、sweepを一つのtransition lockで直列化し、lock中のnetwork I/Oを禁止する。

OpenBriefはAgentごとの小さなstate machineへ落とす。

```text
Stopped
  → Starting
  → Ready
  → Busy
  → Stopping
  → Stopped

Starting / Ready / Busy
  → Failed
```

network接続、auth flow、model request中にglobal lifecycle lockを保持しない。

### PIDだけをidentityにしない

Buzzは次を組み合わせる。

- child handle / PID
- Desktop instance ID
- unpredictable start nonce
- effective configのspawn hash
- runtime receipt

PID再利用や、古いchildから届いたlate lifecycle eventをcurrent processと誤認しないためである。

OpenBriefのMVPは`RuntimeGeneration(UUID)`とchild handleだけから始める。restart badgeやcrash adoptionが必要になった時にconfig hashとreceiptを追加する。

### process treeを所有する

BuzzはUnixでprocess group、WindowsでJob Objectを使い、harnessだけでなく、その下のAgentとMCP serverも終了できるようにする。

OpenBriefでもAgent adapterがさらにCLIやMCP subprocessを起動する前提でtree cleanupをtestする。Linux-only MVPではUnix process groupから開始し、Tauriをcross-platform化する時にWindows Job Objectを追加する。

### readinessとspawnを分ける

binaryがPATHにあること、adapter version、元CLIの有無、authentication済みか、backend daemonがreadyかは別状態である。

```text
Installed
Configured
Authenticated
BackendReady
RuntimeReady
```

OpenBrief MVPはこれを巨大なwizardにせず、起動前checkの失敗理由を一つ返す。自動installerは作らない。

## Agent catalog

Buzzはruntimeを三層へ分ける。

| Tier | Buzz | OpenBrief判断 |
|---|---|---|
| built-in | Goose、Claude、Codex、Buzz Agent | 最初はHermes一つだけ |
| preset | Cursor、OpenCode、Hermes、OpenClaw等 | 価値確認後に静的catalog |
| custom | user JSON command / args / env | 三つ目のruntime要求後 |

固定snapshotで確認した主なentry:

| Agent | Buzz command | 接続形態 |
|---|---|---|
| Hermes Agent | [`hermes-acp`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/desktop/src-tauri/src/managed_agents/discovery.rs#L1595-L1603) | PATH上の外部ACP command |
| OpenClaw | [`openclaw acp`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/desktop/src-tauri/src/managed_agents/discovery.rs#L1604-L1619) | Gateway-backed bridge |
| Codex | [`codex-acp`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/desktop/src-tauri/src/managed_agents/discovery.rs#L139-L170) | Codex App Server adapter |
| Claude Code | [`claude-agent-acp`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/desktop/src-tauri/src/managed_agents/discovery.rs#L107-L137) | Claude Agent SDK adapter |

OpenBriefでは`AgentRuntime`と`LlmProvider`を同じcatalogへ入れない。

```text
AgentRuntime
  Hermes / OpenClaw / Codex / Claude
  stateful session、tool、permission、cancel

LlmProvider
  LM Studio / OpenAI-compatible endpoint
  bounded model task
```

Hermes自身がACPとOpenAI-compatible APIを両方提供しても、OpenBrief内では別adapterとして登録する。

## Agent固有の注意

### Hermes Agent

Buzzの固定snapshotから確認できるのは、PATH上の外部`hermes-acp` commandを引数なしでspawnするpresetまでである。Hermesのcron、Gmail / Slack tool、HTTP送信能力はBuzzのsourceからは保証できないため、実装時にHermesのversionを固定して別途検証する。

OpenBriefの責務としては、scheduled collectionとinteractive ACPを分ける。

```text
Hermes cron / gateway
  Gmail / Slack収集
  ObservationBatch + Brief候補を投入

Hermes ACP
  Desktopからの対話
  Brief深掘り
  natural-language triage
```

cron jobの作成・実行をDesktop ACP sessionへ依存させない。

### OpenClaw

Buzzのpresetは`openclaw acp`がOpenClaw Gatewayへ接続し、tool実行はDesktop processではなくGateway側で起きると明記する。

このためDesktopがchildへ設定したOpenBrief credentialやMCP commandは、Gateway execution environmentへ自動伝播しない。OpenClaw採用時はOpenBrief MCP / tokenをGateway側へ明示的に設定する。

### Codex

Codex CLIを直接ACP Agentとして扱わず、[`agentclientprotocol/codex-acp`](https://github.com/agentclientprotocol/codex-acp)がCodex App Server eventをACPへ変換する。

Buzzには旧`@zed-industries/codex-acp`と新adapterのversion差を扱うcompatibility codeがある。OpenBriefは旧adapterをsupportせず、実装時点の現行packageだけをminimum version付きで扱う。

### Claude Code

Claude Code CLIのnative ACPを前提にせず、[`agentclientprotocol/claude-agent-acp`](https://github.com/agentclientprotocol/claude-agent-acp)をadapterとして扱う。

adapterがClaude Agent SDKを利用するため、Claude Codeの全CLI挙動と同一とは仮定しない。session load、permission、MCP等はACP capability resultとintegration testで確認する。

## ACP、MCP、ingress、Model Gatewayを分ける

| Boundary | Direction | 用途 |
|---|---|---|
| ACP | Desktop ↔ Agent | chat、stream、tool、permission、cancel |
| MCP | Agent → OpenBrief | Brief query、triage候補、限定されたlocal action |
| Observation ingress | Hermes cron等 → OpenBrief | scheduled collection resultの冪等投入 |
| Model Gateway | OpenBrief → LM Studio等 | classification、summary、screen observation |

ACPをOpenAI-compatible `/v1/chat/completions`へ偽装しない。逆にLLM Provider responseをACP tool eventとして捏造しない。

remote ACP transportは固定snapshot時点で発展中であり、OpenBrief MVPはlocal stdioを基準にする。remote Agentは、local bridge commandがremote Gatewayへ接続する場合だけcatalogへ入れる。

### OpenBrief MCPのownership

MVPのMCP serverは`openbrief mcp serve`という同一binaryのstdio modeにする。Tauri Rustがtrusted configからcommand、store path、read / reversible-triage scopeを組み立て、ACPの`session/new.mcpServers`へ渡す。Agentが任意のcommand、database path、scopeを指定する形にはしない。

tool inputとresultにはbyte上限を置き、queryはbounded read modelだけを返す。triage writeはappend-onlyのUserDecisionまたは元に戻せるstate transitionへ限定する。external write toolは公開しない。stdio subprocessはAgent runtimeのprocess treeと一緒に終了する。

これはlocal stdioだけの境界であり、Bearer tokenを追加しない。OpenClawのようにtoolが別Gatewayで動くruntimeを追加する場合は、localhost / Tailnet endpoint、認証、scope、retentionを別設計にする。

## scheduled producerからObservationを受ける境界

最初の価値検証では、Gmail / Slack adapterをOpenBriefへ実装しない。

```text
Hermes cron等のscheduled producer
  Gmail / Slack read-only tools
        ↓
  ObservationBatch
  + optional Brief candidate
        ↓ authenticated ingress
OpenBrief local store
        ↓
Tauri Brief view
        ↕ ACP
Hermes interactive session
```

producer側に残すもの:

- Gmail / Slack credential
- source API pagination / cursor
- cron schedule
- source固有retry
- raw API responseの一時処理

OpenBriefへ渡すもの:

- producer identity
- observation window
- generated / fetched timestamp
- normalized observationとsource reference
- optional natural-language Brief candidate
- source freshness / coverage
- content sensitivity

初期envelopeの例:

```json
{
  "schema_version": 1,
  "producer": "hermes",
  "observed_from": "2026-07-30T00:00:00+09:00",
  "observed_to": "2026-07-30T12:30:00+09:00",
  "generated_at": "2026-07-30T12:31:00+09:00",
  "observations": [
    {
      "source": "gmail",
      "source_id": "thread-123",
      "occurred_at": "2026-07-30T09:14:00+09:00",
      "summary": "木曜の実験枠について確認が届いている"
    },
    {
      "source": "slack",
      "source_id": "C123:456",
      "occurred_at": "2026-07-30T11:52:00+09:00",
      "summary": "利用中crateのsecurity advisoryが共有されている"
    }
  ],
  "proposed_brief_markdown": "今日扱う候補は2件です。"
}
```

これは実装前の最小例であり、固定APIではない。accepted ADRではnormalized Observationがcanonical ingressである。Hermes等が自然文Briefまで生成しても、それは`proposed_brief_markdown`というderived candidateとして扱い、source observationと本人の判断を置き換えない。

最初のprototypeでBrief candidateだけを受け取るfast pathを選ぶ場合は、accepted ADRからの実験的逸脱として別途記録する。chat transcriptやHermes memoryをOpenBriefのauthorityにしない。

HermesのGmail / Slack / cron能力はこのBuzz調査では未検証である。P1はfixtureまたは任意のschema producerで成立させ、Hermesを必須dependencyにしない。実接続時にHermesのversion、利用可能tool、read-only scope、schedule時のtool availability、ingress送信方法を固定して確認する。

email、Slack message、web contentはAgentへの命令ではなくuntrusted evidenceとして渡す。外部への返信、calendar書き込み、message削除はBrief ingestと同じcapabilityにしない。

## OpenBriefへ採る最小crate境界

最初からcrateを全て作らず、実装する段階で次の責務へ分ける。

```text
openbrief-agent-api
  AgentRuntimeId、capability、session / update type

openbrief-agent-acp
  stdio ACP client、timeout、cancel

openbrief-runtime
  allowlisted process spawn、generation、cleanup

openbrief-mcp
  Agent向けread / reversible triage tool、stdio server lifecycle

openbrief-ingress
  ObservationBatch validation、idempotency、provenance

openbrief-desktop
  Tauri command / event adapter
```

`openbrief-agent-api`と`openbrief-agent-acp`を最初の一crateにしてもよい。`openbrief-mcp`も最初は既存binary内のsubcommandとして起動し、独立配布しない。二つ目のprotocol implementationまたは独立test boundaryが必要になるまで、pathologicalなmicro-crate分割をしない。

## 採るpattern

| Pattern | 採用 |
|---|---|
| Rust側がchild processとsecretを所有 | 採る |
| typed Tauri command / event | 採る |
| ACP initialize / capability negotiation | 採る |
| Buzz固有ACP v2 / `_meta` extension | 採らない |
| permissionの`allow_once`自動承認 | 採らない |
| stdout protocol、stderr diagnostic | 採る |
| input byte上限、idle / absolute timeout | 採る |
| cancel、process tree kill、wait | 採る |
| runtime generation ID | 採る |
| readinessをinstalled / auth / backendへ分ける | 小さく採る |
| static runtime catalog | Hermes一つから開始 |
| custom harness JSON | 複数runtime後 |
| process pool、queue、heartbeat | 負荷を実測した後 |

## 採用しない範囲

| 対象 | 理由 |
|---|---|
| Buzz repository dependency / fork | domainとdeploymentが大きく異なる |
| Nostr event model | local personal Briefに不要 |
| Buzz relay | OpenBrief local storeがauthority |
| Postgres / Redis / MinIO | single-user MVPに過剰 |
| agent Nostr identity | OS userとruntime configで足りる |
| channel / team / persona | finite Brief triageから逸れる |
| `buzz-acp` process pool | Hermes一つのinteractive sessionには不要 |
| channel queue / mention batching | cron ingressとmanual chatを分離する |
| workflow engine | Hermes / OpenClaw cronを再利用する |
| auto installer | PATHとreadinessを表示するだけでよい |
| mesh LLM provider | x870 LM StudioをModel Gatewayから使う |
| full `acp.rs` copy | Buzz固有extensionとenv mergeが多い |

## 実装順

### P1: Brief data plane

1. `ObservationBatch`とoptional Brief candidateをfixtureで固定する
2. stdinまたはauthenticated local ingressで冪等importする
3. SQLiteへObservation、source reference、producer、freshnessを保存する
4. Policy / Model Gatewayを通して有限Briefを生成し、`openbrief briefs --json`でbounded read modelを返す

### P2: Tauri one-screen

1. 最新の有限Briefを表示する
2. source freshnessと根拠だけ展開できる
3. 自然言語triage inputを一つ置く
4. external writeはまだ持たない

### P3: Hermes ACP

1. `hermes-acp`一つをallowlistする
2. initialize、session/new、prompt、update、cancelだけ実装する
3. `openbrief mcp serve`をstdio serverとして実装し、bounded readとreversible triage toolだけ公開する
4. MCP subprocessをACP sessionと同じprocess treeで起動・停止する
5. tool resultのbyte上限、default denyのpermission broker、external write不可をintegration testする
6. Hermes連携時はversionとcron tool availabilityを固定して検証し、scheduled collectionをDesktop ACP sessionから独立させる

### P4: runtime追加

価値確認後にCodex、Claude、OpenClawの順でadapter integration testを追加する。共通UIへ押し込まず、capability差を表示する。

### P5: Activity source

foreground timelineやscreen summaryを別Observation producerとしてBriefへ接続する。ACP導入を理由にraw screenを全Agentへ公開しない。

## 再調査する条件

次のどれかが発生するまでは、固定SHA `63496cc…`を基準にしてBuzz全体を再調査しない。

1. Tauriで二つ目のAgent runtimeを追加する
2. child process treeのcleanup bugが発生する
3. runtime crash adoption / auto-restartが必要になる
4. custom Agent runtimeをuserが登録する
5. 複数同時ACP sessionを提供する
6. Agent event queue、backpressure、heartbeatが必要になる
7. OpenClaw Gateway bridgeを実装する
8. Windows / macOSへDesktopを配布する
9. remote ACP transportを正式採用する
10. Buzz codeを直接移植する
