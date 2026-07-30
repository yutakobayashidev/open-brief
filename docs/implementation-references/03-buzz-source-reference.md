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
| Mobile追補SHA | [`61b96c9828d1dd54106b570d87a54edbc92bb9c4`](https://github.com/block/buzz/tree/61b96c9828d1dd54106b570d87a54edbc92bb9c4) |
| Mobile追補日 | 2026-07-31 |
| License | [Apache License 2.0](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/LICENSE) |
| 調査日 | 2026-07-30 |

codeを移植する場合はApache-2.0のcopyright、license、NOTICE要件を確認し、変更したfileへ変更の明示を残す。現時点では直接移植せず、source pathとpatternだけを参照する。

BuzzのACP clientはBuzz固有のrelay、Codex環境変数merge、Goose / Claude extension、usage accountingまで含む。OpenBriefでは公式ACP Rust libraryまたは最小clientを先に評価し、`acp.rs`全体をコピーしない。

Codex配布の判断には、現行[`agentclientprotocol/codex-acp`](https://github.com/agentclientprotocol/codex-acp/tree/ba5bef59cfcea4229841fe9438d816696621307b)も補助資料として使った。調査時点のpackageは`1.1.7`、lockfile上の`@openai/codex`は`0.145.0`である。OpenBriefの`flake.lock`が固定するnixpkgs `624af665…`の`codex-acp`は、別repositoryの旧Zed版`0.13.0`であり、現行adapterの代替にはならない。

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
  └─ Unix socket client
       └─ openbriefd
            └─ ACP Agent subprocess

OpenBrief local store
  └─ Brief / source reference / UserDecision authority
```

現行Desktop MVPでは同じ`openbriefd`を、`systemd --user`が所有するheadless daemon版と、Tauriがmanaged childとして所有するApp版の両方で使う。TauriはACP runtimeを直接所有しない。詳細は[Desktop Agent MVPのtarget deployment](../architecture/desktop-agent-mvp.md#target-deployment-daemon版とapp版)に固定する。

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

## Flutter MobileとACPの接続

### 結論

Buzz MobileはACPへ直接接続しない。Flutter appはNostr clientとしてRelayへ接続し、別hostで動く`buzz-acp`がRelay eventを受けてlocal ACP subprocessを操作する。

```text
Buzz Flutter Mobile
  │  kind 9: @agentを含む署名済みmessage
  ▼
Buzz Relay
  │  WebSocket subscription
  ▼
buzz-acp（desktop、server等の常時稼働host）
  │  stdio ACP: initialize / session/new / session/prompt
  ▼
Codex / Claude / Goose等
  │  buzz messages send
  ▼
Buzz Relay
  │  通常のchannel message
  ▼
Buzz Flutter Mobile
```

[`mobile/pubspec.yaml`](https://github.com/block/buzz/blob/61b96c9828d1dd54106b570d87a54edbc92bb9c4/mobile/pubspec.yaml#L9-L44)にはWebSocket、Nostr、secure storage、暗号libraryがあるが、ACP SDKはない。したがって「mobileからACPへremote接続」ではなく、「Relayをdomain transportとして共有し、その裏でharnessがACPをlocal実行する」構成である。

### promptと最終回答

Mobileの[`SendMessage`](https://github.com/block/buzz/blob/61b96c9828d1dd54106b570d87a54edbc92bb9c4/mobile/lib/features/channels/send_message_provider.dart#L42-L89)は次を行う。

1. channelを`h` tag、mention先を`p` tagへ入れる
2. kind 9 eventをuserのNostr private keyで署名する
3. NIP-42認証済みWebSocketへpublishし、Relayの`OK`を待つ

`buzz-acp`はRelay subscription上でauthor policyとsubscription ruleを確認し、eventをchannel queueへ入れてACPの`session/prompt`へ変換する。AgentのACP出力は直接channel messageにならない。[`agent_message_chunk`](https://github.com/block/buzz/blob/61b96c9828d1dd54106b570d87a54edbc92bb9c4/crates/buzz-acp/src/acp.rs#L1714-L1720)はstream log / observerへ流れるだけで、最終回答はAgentが[`buzz messages send`](https://github.com/block/buzz/blob/61b96c9828d1dd54106b570d87a54edbc92bb9c4/crates/buzz-acp/src/base_prompt.md#L64-L74)を実行して通常messageとしてRelayへ投稿する。

このためMobileはACP session IDやAgent processを所有しない。通常のchannel subscriptionを再接続時にreplayし、回答messageを他の参加者の投稿と同じ方法で受信する。

### 途中経過はACP transportではなくObserver mirror

BuzzはACPのtool call、thought、message chunk等をMobileへ見せるため、NIP-AOのkind 24200を別に使う。

```text
ACP notification
  → buzz-acp observer
  → NIP-44 encrypted kind 24200
  → Relayのin-memory pub/sub
  → Mobileで復号してactivity UIへ表示
```

[`NIP-AO`](https://github.com/block/buzz/blob/61b96c9828d1dd54106b570d87a54edbc92bb9c4/docs/nips/NIP-AO.md#L18-L70)では、Agentとownerの間でtelemetry / control frameをNIP-44暗号化する。これはephemeral eventであり、Relayは永続化・検索・audit log投入をしてはならない。[`buzz-acp`](https://github.com/block/buzz/blob/61b96c9828d1dd54106b570d87a54edbc92bb9c4/crates/buzz-acp/src/lib.rs#L790-L832)がAgent側で暗号化・署名してpublishし、Mobileの[`observer_subscription.dart`](https://github.com/block/buzz/blob/61b96c9828d1dd54106b570d87a54edbc92bb9c4/mobile/lib/features/channels/agent_activity/observer_subscription.dart#L109-L217)はowner宛てframeだけを検証・復号し、Agentごとに最大800件をmemoryへ保持する。

NIP-AO自体はownerからAgentへの`cancel_turn`も定義し、`buzz-acp`はcontrol frameの署名、owner、freshnessを再検証してcancelへ変換する。ただし追補snapshotのFlutter実装で確認できるのはobserverの受信側であり、Mobileからcontrol frameを送るUI / publisherはまだ見当たらない。Mobileで表示できることと、MobileからACP lifecycleを完全操作できることは分けて評価する。

### pairingとcredential

Mobile onboardingはNIP-ABの`nostrpair://` QRを使う。ephemeral key、共有secret、SAS照合、暗号化payloadでdesktop側から`relayUrl`、`pubkey`、`nsec`を受け取り、RelayへNIP-42接続できることを検証してから保存する。[`pairing_provider.dart`](https://github.com/block/buzz/blob/61b96c9828d1dd54106b570d87a54edbc92bb9c4/mobile/lib/features/pairing/pairing_provider.dart#L448-L481)と[`community_storage.dart`](https://github.com/block/buzz/blob/61b96c9828d1dd54106b570d87a54edbc92bb9c4/mobile/lib/shared/community/community_storage.dart#L19-L28)から、同じNostr `nsec`をMobileのsecure storageへ移す設計だと分かる。

これはBuzzではMobile自身がmessageへ署名するために必要だが、OpenBriefがそのまま採る理由はない。Codex login token、Agent credential、Desktopのmaster keyをMobileへ複製してはならない。

### OpenBrief Flutterへの判断

OpenBriefはBuzz Relayではなくlocal SQLiteをauthorityとする。現行MVPではACP processを`openbriefd`が所有する。将来のFlutter clientにも、このdaemonをremote application boundaryとして使う。

```text
Flutter companion
  │  authenticated domain command / event stream
  ▼
openbriefd（x870またはdesktop）
  ├─ OpenBrief local authority
  ├─ ACP runtime / session owner
  └─ VLM / source producer
```

Mobileへ公開するのはraw ACP JSON-RPCではなく、`BriefUpdated`、`AgentStatusChanged`、`TurnDelta`、`ProposalReceived`、`DecisionConfirmed`等のOpenBrief domain eventと、対応するcommandにする。これによりACP adapter、Agent種別、session ID、permission broker、process cleanupをserver側の実装詳細として保てる。

最初のFlutter companionは次へ限定する。

1. QRでx870 / desktopの`openbriefd`へdevice pairingする
2. 有限Brief最大3件とReturn Anchorを読む
3. 自然言語dumpを送り、Agentの提案をforegroundでstream表示する
4. 「今日扱う / 扱わない / 戻る」を本人がconfirmする
5. 切断後はcursorから再同期し、同じconfirmをidempotency keyで重複適用しない

認証はdevice固有keyとrevoke可能なcredentialにし、Tailscaleは到達性とdevice identityの一層として利用する。画面frameはx870側に残し、Mobileへは要約と明示要求された最小evidenceだけを返す。iOS / Androidのbackground socketをAgent lifecycleの前提にせず、foregroundのevent streamと、後続の完了push notificationを分ける。

Buzzから採るべき中核はRelayそのものではなく、次の二つである。

- Agent runtimeをMobileから分離し、常時稼働hostへ閉じる
- durableな最終状態とephemeralな実行telemetryを別contractにする

### `buzz-acp`の常駐化はDesktop-owned child process

ここでいう常駐は、`systemd` / `launchd`へ登録された独立daemonではない。Buzz Desktopが起動中のmanaged runtimeとして`buzz-acp`を所有する。

```text
Tauri Desktop
  ├─ AppState.managed_agent_processes（Child handle / runtime key）
  ├─ managed-agents.json（設定・start_on_app_launch）
  └─ runtime receipt（PID、instance、started_at）
       ↓
     buzz-acp child
       ↓
     ACP Agent process pool
```

ライフサイクルは次の通りである。

1. Desktop boot時はすぐspawnせず、active workspaceのRelayとidentityが決まるまで`managed_agent_restore_pending`を立てる
2. workspace apply後、`start_on_app_launch=true`のlocal agentだけを復元する。既存PID、receipt、孤児processを先に検査する
3. `Command`で`buzz-acp`をspawnし、`BUZZ_PRIVATE_KEY`、Relay URL、Agent command、MCP commandなどを環境変数で注入する
4. Unixではharnessを独立process group、WindowsではJob Objectへ入れ、ACP配下のMCP / Agent subprocessも同じ単位で管理する
5. `buzz-acp`はRelay WebSocketをlistenし、channel eventをqueueへ入れる。ACP Agent自体はpool内で eagerまたはlazyに起動する
6. Desktop終了時はin-flight turnにgrace periodを与え、process group / Job Objectを停止する。別instanceの残骸はboot時と60秒周期のsweepで回収する

根拠は、[workspace apply後のrestore](https://github.com/block/buzz/blob/61b96c9828d1dd54106b570d87a54edbc92bb9c4/desktop/src-tauri/src/commands/workspace.rs#L236-L280)、[auto-start候補の選別](https://github.com/block/buzz/blob/61b96c9828d1dd54106b570d87a54edbc92bb9c4/desktop/src-tauri/src/managed_agents/restore.rs#L166-L189)、[child spawnとprocess group](https://github.com/block/buzz/blob/61b96c9828d1dd54106b570d87a54edbc92bb9c4/desktop/src-tauri/src/managed_agents/runtime.rs#L879-L958)、[harnessのRelay loop / pool起動](https://github.com/block/buzz/blob/61b96c9828d1dd54106b570d87a54edbc92bb9c4/crates/buzz-acp/src/lib.rs#L1295-L1321)にある。

したがって、Buzz Desktopを終了すればlocal `buzz-acp`も原則終了する。Mobileから使えるのは、Desktopまたは別途稼働させたremote backendがRelayへ接続している間であり、MobileがACPを起こしているわけではない。

OpenBriefでは同じ`openbriefd`を、Tauri Desktop起動中だけのmanaged-child modeと、x870等で`systemd --user`が常時稼働させるdaemon modeに分ける。Flutterから直接ACPへ接続せず、`openbriefd`がACP childとsessionを所有する。

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
| [`commands/agent_discovery.rs`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/desktop/src-tauri/src/commands/agent_discovery.rs) | install lock、install plan、retry、再検出 | 状態遷移だけ採る |
| [`commands/agent_discovery/managed_node.rs`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/desktop/src-tauri/src/commands/agent_discovery/managed_node.rs) | checksum付きprivate Node/npm | app-private配布の参考 |
| [`commands/agent_discovery/post_install_verification.rs`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/desktop/src-tauri/src/commands/agent_discovery/post_install_verification.rs) | install後の再検出と検証 | 採る |
| [`commands/agent_auth.rs`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/desktop/src-tauri/src/commands/agent_auth.rs) | ACP auth method取得と認証開始 | 採る |
| [`SetupStep.tsx`](https://github.com/block/buzz/blob/63496cc1d4c6f1b7c613801bdcc694169dcf391a/desktop/src/features/onboarding/ui/SetupStep.tsx) | 初回runtime選択、install、login、polling | 一枚のCodex cardへ縮小 |
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

OpenBrief MVPはこれを巨大なwizardにしない。ただし、設定fileへ絶対pathを書かせるだけでは初回体験として不足する。adapterをapplicationまたはNix packageが供給し、画面には次の一手を一つ返す。

## Onboardingとmanaged agentの深掘り

### Buzzの初回体験は「検出、導入、認証」を分離している

Buzzのcatalogは単なる`which`の結果ではない。主に次の直交する状態を返す。

```text
Availability
  Available
  AdapterMissing
  AdapterOutdated
  CliMissing
  NotInstalled

Authentication
  LoggedIn
  LoggedOut
  ConfigInvalid
  NotApplicable
  Unknown
```

初回画面はClaude、Codex、Goose、Buzz Agentのcardを表示し、一つでもreadyなら次へ進める。skipも常に可能である。Codexのadapterが利用可能だが未loginの場合はACPからauth methodを取得し、初回ではChatGPT loginを優先する。認証開始後は2秒ごと、最大120秒catalogを再取得する。

この分離により、`binary not found`、古いadapter、元CLI不足、未login、壊れたCodex configを同じ「接続失敗」に潰していない。OpenBriefも`codexを確認中`という一状態ではなく、availability、authentication、process lifecycleを別fieldで返すべきである。

### Buzzのinstallerはglobal npmに見えてapp-privateである

Codexのcatalogには次が記載されている。

```text
CLI install
  official Codex install script

ACP install
  npm install -g @agentclientprotocol/codex-acp
```

ただし実行時にはBuzzが固定Node `v24.18.0`をapp dataへ展開し、npmのglobal prefixも`Buzz/node-tools`へ書き換える。archiveはplatformごとのSHA-256、90 MiB上限、path traversal検査、temporary directoryからのrename、旧directoryへのrollbackを持つ。install commandは5分でtimeoutし、通常の非zero exitだけ最大3回retryする。runtime単位のinstall lockにより同時installも防ぐ。

install後はPATHとcatalog cacheを更新し、最終状態が`Available`になるまで成功扱いにしない。setup modeで待機していた同runtimeのprocessだけを再起動する。

これはsystemのnpmやPATHを汚さない点でよい。一方で、次はそのまま採らない。

- vendor CLIの`curl | shell`はremote scriptを実行し、再現性が低い
- ACP install commandはpackage versionを固定せず、minimum versionを満たす最新版を取得する
- Node runtime、npm tree、vendor CLIの三層を初回にnetwork installするため失敗面が広い

### 未準備でもsetup listenerを起動する

BuzzはruntimeがNotReadyでも通常のAgent poolを起動せず、Desktopだけが生成できるsetup payloadを渡した最小`buzz-acp` listenerを起動する。これにより、作成済みAgentは設定不足をUIへ示し、install完了後に再起動できる。

OpenBriefではこのための別processは不要である。Tauri core自身が`RuntimeStatus`を保持し、未準備時はACP subprocessを起動しない。`agent_start`は初期状態を同期的に返し、その後の変化だけeventで通知する。これならfrontendがlistener登録前の最初のeventを失って「確認中」のままになるraceも避けられる。

### 認証はadapterが広告するmethodを使う

Buzzは`codex login status`を軽量probeに使い、必要な時だけbundled helperからACPへ接続して`auth-methods`を取得する。選択したmethodが現在も広告されていることを検証してから、terminal型は見えるterminalで、非terminal型はACP `authenticate`で開始する。Codex config parse errorはlogged-outと区別する。

OpenBriefでもChatGPT loginをhard-codeしたshell commandとして扱わない。

```text
discover adapter
  → initialize
  → auth methods
  → user selects / recommended method
  → authenticate
  → bounded readiness polling
```

secretはOpenBrief Storeへ複製しない。Codex自身のcredential storeを使い、OpenBriefは`AuthRequired | Ready | ConfigInvalid`だけを保持する。

## Nixと非Nixで共通にする配布設計

### 結論

Nix用と通常配布用でAgent接続処理を二つ作らない。同じversion、同じcapability、同じauth flowの`codex-acp` sidecarを使い、実行物の供給元だけを変える。

```text
OpenBrief AgentRuntime
  └─ codex-acp 1.1.7-compatible executable
       ├─ Nix: $out/libexec/openbrief/codex-acp
       └─ non-Nix: Tauri resource/libexec/codex-acp
```

現行`codex-acp`はnpm package内にcompatibleな`@openai/codex`を含み、`CODEX_PATH`は別Codexを明示的に使う場合だけ必要である。repositoryにはBunでLinux、macOS、Windowsのx64 / arm64 standalone binaryを作るscriptもあるが、`v1.1.7`で実測すると`codex-acp cli --help`がBun filesystem内から`@openai/codex/bin/codex.js`を解決できなかった。外部`CODEX_PATH`を要求すると同梱目的を失うため、現時点ではNode packageを使う。

### Nix

- nixpkgsの`codex-acp`は旧Zed版`0.13.0`なので使わない
- OpenBrief flakeで現行adapterのsource、lockfile、versionを固定した独自packageを公開する
- `openbrief` packageはadapterを`$out/libexec/openbrief/codex-acp`に含め、PATHへ依存しない
- `packages.openbrief-codex-acp`も公開し、Home Manager moduleではpackageまたはpathを上書き可能にする
- Nix Store内では自己更新しない。UIには`Managed by Nix`と表示し、更新は`nix flake update`とrebuildに委ねる
- auth stateはuserのCodex homeに置き、immutableなStoreへ書かない

Nix buildでは公式release tag `v1.1.7`とpackage lockを`buildNpmPackage`へ固定し、Node applicationとしてbuildする。`codex-acp --version`だけでなく、内蔵Codexを通る`codex-acp cli --help`もpackage testに含める。outputは`$out/libexec/openbrief/codex-acp`から内部解決し、cross-platform bundleを一つのderivationで作らない。

### 非Nix

第一候補はOpenBrief release CIで同じ固定Node packageとproduction dependency tree、対応Node runtimeをTauri resourceへ同梱することである。初回起動時のdownloadもNode/npm導入もなく、onboardingはほぼ認証だけになる。Nix Storeを参照するwrapperを通常版へ流用せず、非Nix release環境でportableなresourceを組み立ててsmoke testする。

同梱がsizeやrelease cadenceの実測上問題になった場合にだけ、Buzz型のmanaged installを第二候補にする。その場合も次を満たす。

- app dataのversion付きprivate directoryへ置く
- exact version、URL、digestをOpenBrief release manifestへ固定する
- temporary download、digest検証、atomic rename、rollbackを行う
- global npm、system PATH、`curl | shell`を変更・実行しない
- silent installせず、ユーザーの明示操作で行う
- install後に同じcapability probeで再検証する

### 共通contract

最初から汎用plugin systemは作らない。Codex一つに必要な最小contractは次で足りる。

```text
RuntimeSource
  Packaged
  NixStore
  Managed
  Override

RuntimeStatus
  Missing
  Installing
  Incompatible
  AuthRequired
  Ready
  Failed

RuntimeReceipt
  runtime_id
  adapter_version
  source
  executable_path
  digest
  last_verified_at
```

解決順は`explicit override → packaged / Nix libexec → managed sidecar`とする。任意のPATH binaryはMVPでは自動採用しない。sourceを切り替えて自動retryすると、異なるversionやcredential contextへ黙って移るためである。overrideもversionとcapabilityを検証してから起動する。

### OpenBriefのonboarding

初回はCodex card一枚でよい。

```text
Detecting
  → Ready
  → Sign in
  → Install / Repair
  → Incompatible
  → Failed
```

cardにはversionとprovenanceを`Included with OpenBrief`、`Managed by Nix`、`Custom path`のいずれかで表示する。actionは常に一つに絞るが、detailsでは実行path、probe結果、stderrのsanitized summaryを確認できる。認証はBuzz同様にadapterのauth methodを読み、ChatGPT loginを推奨し、advancedで他methodを選べるようにする。`Skip for now`も残す。

## Agent catalog

Buzzはruntimeを三層へ分ける。

| Tier | Buzz | OpenBrief判断 |
|---|---|---|
| built-in | Goose、Claude、Codex、Buzz Agent | 最初はCodex一つだけ |
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
| MCP | Agent → OpenBrief | Brief proposal、triage proposal |
| Observation ingress | Hermes cron等 → OpenBrief | scheduled collection resultの冪等投入 |
| Model Gateway | OpenBrief → LM Studio等 | classification、summary、screen observation |

ACPをOpenAI-compatible `/v1/chat/completions`へ偽装しない。逆にLLM Provider responseをACP tool eventとして捏造しない。

remote ACP transportは固定snapshot時点で発展中であり、OpenBriefはlocal stdioを基準にする。runtime catalogへ入れられるのはstable ACPをstdioで提供するadapterだけとし、remote Gatewayのnative protocolは実装しない。

### OpenBrief MCPのownership

MVPのMCP serverは`openbrief mcp serve`という同一binaryのstdio modeにする。Tauri Rustがtrusted configからcommandとstore pathを組み立て、ACPの`session/new.mcpServers`へ渡す。Agentが任意のcommand、database path、scopeを指定する形にはしない。

公開toolは`brief_propose`と`triage_propose`だけにする。どちらもinert proposalを保存し、UserDecision、CuriosityCapture、ReturnAnchorを直接作れない。本人がDesktopで確認したときだけapplication serviceが確定する。external write toolは公開しない。stdio subprocessはAgent runtimeのprocess treeと一緒に終了する。

これはlocal stdioだけの境界であり、Bearer tokenを追加しない。toolが別Gatewayで動くAgentは、OpenBrief MCPを安全にstdioで渡せないため現行catalogの対象外とする。

## scheduled producerからObservationを受ける境界

最初の価値検証では、Gmail / Slack adapterをOpenBriefへ実装しない。

```text
Hermes cron等のscheduled producer
  Gmail / Slack read-only tools
        ↓
  ObservationBatch
        ↓ authenticated ingress
OpenBrief local store
        ↓
Tauri Brief view
        ↕ ACP
Codex interactive session
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
  "source_coverage": []
}
```

上記は調査時の概念例であり、実装済みschemaは`ObservationBatch` domain型とGC-04 fixtureを正とする。normalized Observationがcanonical ingressであり、Briefはinteractive AgentがMCPからproposalとして作る。chat transcriptやAgent memoryをOpenBriefのauthorityにしない。

HermesのGmail / Slack / cron能力はこのBuzz調査では未検証である。P1はfixtureまたは任意のschema producerで成立させ、Hermesを必須dependencyにしない。実接続時にHermesのversion、利用可能tool、read-only scope、schedule時のtool availability、ingress送信方法を固定して確認する。

email、Slack message、web contentはAgentへの命令ではなくuntrusted evidenceとして渡す。外部への返信、calendar書き込み、message削除はBrief ingestと同じcapabilityにしない。

## OpenBriefへ採る最小crate境界

最初からcrateを全て作らず、実装する段階で次の責務へ分ける。

```text
openbrief-agent
  公式ACP SDK、allowlisted process、generation、timeout、cancel、cleanup

openbrief-app
  Observation ingress service、proposal-only MCP、本人確認

openbrief-store
  Observation、proposal、本人状態のSQLite authority

openbrief-desktop
  Tauri command / event adapter
```

MCPは既存CLIのhidden subcommandとして起動し、独立crateや独立配布にしない。二つ目のprotocol implementationまたは独立test boundaryが必要になるまで、pathologicalなmicro-crate分割をしない。

## 採るpattern

| Pattern | 採用 |
|---|---|
| Rust側がchild processとsecretを所有 | 採る |
| typed Tauri command / event | 採る |
| ACP initialize / capability negotiation | 採る |
| Buzz固有ACP v2 / `_meta` extension | 採らない |
| permissionの`allow_once`自動承認 | proposal-only toolに限り採る |
| stdout protocol、stderr diagnostic | 採る |
| input byte上限、idle / absolute timeout | 採る |
| cancel、process tree kill、wait | 採る |
| runtime generation ID | 採る |
| readinessをinstalled / auth / backendへ分ける | 小さく採る |
| static runtime catalog | Codex一つから開始 |
| adapterをapp-privateに供給 | Nix libexec / Tauri sidecarとして採る |
| install後のversion / capability再検証 | 採る |
| adapter広告型のauth method | 採る |
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
| global npm / PATHを変更するinstaller | app同梱sidecarを優先する |
| remote vendor scriptの実行 | reproducibilityとsupply-chain境界が弱い |
| mesh LLM provider | x870 LM StudioをModel Gatewayから使う |
| full `acp.rs` copy | Buzz固有extensionとenv mergeが多い |

## Transport再評価

2026-07-31にACP stable v1、Codex app-server、Buzz Mobileを再比較した。

| Boundary | Reference behavior | OpenBrief decision |
|---|---|---|
| ACP | stdio上のJSON-RPCでinitialize、session、stream、cancelを扱う | `openbriefd`内部のAgent境界だけに使う |
| Codex app-server | stdio / WebSocket / Unix socketでCodex固有thread・turn APIを公開する | backpressureやlifecycleだけ参考にし、protocolは採らない |
| Buzz Mobile | Relay WebSocketでdomain messageを扱い、ACPへ直接接続しない | MobileとACP processを分離する判断だけ採る |
| OpenBrief local | CLI / Tauriからdaemonへ短いone-shot操作を送る | Unix socket上のtyped JSONを維持する |
| OpenBrief remote | snapshot、turn開始、proposal確認、domain eventが必要 | HTTPS command＋server-to-client WebSocketを維持する |

Codex app-serverのJSON-RPCはrich Codex clientのためのpublic APIであり、ACPの代替ではない。OpenBriefがこれを採るとCodexのthread、approval、auth schemaを再公開することになる。逆にlocal controlをJSON-RPC化しても、一接続一request / responseの現状ではID correlationやserver requestを利用しない。

したがってprotocolの見た目を統一せず、authorityとfailure modelをそろえる。詳細は[ADR 0004](../adr/0004-separate-acp-local-and-remote-transports.md)に記録した。

## 実装順

### P1: Brief data plane（実装済み）

1. `ObservationBatch`をGC-04 fixtureで固定する
2. stdinまたはJSON fileから冪等importする
3. SQLiteへObservation、source reference、producer、freshnessを保存する
4. Agentの`brief_propose`から有限Brief proposalを保存する

### P2: Tauri one-screen（実装済み）

1. 最新の有限Briefを表示する
2. source freshnessと根拠だけ展開できる
3. 自然言語triage inputを一つ置く
4. external writeはまだ持たない

### P3: Codex ACP（実装済み）

1. 静的runtime catalogで`codex-acp`一つをallowlistし、絶対pathはadvanced overrideにする
2. initialize、session/new、prompt、update、cancelだけ実装する
3. `openbrief mcp serve`をstdio serverとして実装し、proposal-only toolを二つだけ公開する
4. MCP subprocessをACP sessionと同じprocess treeで起動・停止する
5. tool resultのbyte上限、default denyのpermission broker、external write不可をintegration testする
6. Observation snapshotを100件、64 KiBまでに制限する

### P3.1: Codex onboardingと配布（Nix版とcatalog化を実装済み）

1. 現行`agentclientprotocol/codex-acp`のversionとsourceをOpenBrief側で固定する
2. Nix packageを`$out/libexec/openbrief`へ、通常版をTauri resourceへ同梱する
3. configの絶対path必須をadvanced overrideへ下げる
4. availability、auth、process stateを分けたstatus responseを同期的に返す
5. adapterからauth methodを取得し、login後のbounded pollingを実装する
6. Nix buildと通常release artifactへ同じadapter contract testを通す

Buzzの`KnownAcpRuntime`をそのまま移植せず、Desktopの`AcpRuntimeSpec`へprovider ID、表示名、同梱path、引数、version policy、認証方法の優先hint、OpenBrief MCPの付与可否だけを保持する。resolver、probe、認証、起動はこのspecを入力とする共通flowにし、provider変更時は既存runtimeを停止する。dynamic JSON harness、install shell、PATH discovery、process poolは価値確認前には採らない。通常release artifactへのportable Node runtime同梱は未実装である。

### P4: scheduled producer

Hermes / OpenClaw等のcronから、同じObservationBatch ingressへGmail / Slackのread-only結果を投入する。interactive Codex sessionとschedule lifecycleを結合しない。

### P5: ACP runtime追加

価値確認後もOpenBriefが対応するAgent protocolはACPだけとする。別Agentを追加する場合はstable ACPを満たすadapterとintegration testを要求し、Hermes、OpenClaw、Codex app-server等のnative protocol clientは実装しない。共通UIへcapabilityを押し込まず、ACP initialize responseの差として表示する。

### P6: Activity source

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
