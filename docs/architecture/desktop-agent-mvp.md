# Desktop Agent MVP

上位の製品定義と、Activity Recall・Attention Triage・自然言語入力の関係は[Attention Control Plane design memo](attention-control-plane.md)にまとめています。

## Outcome

このMVPは、Observationを眺めるdashboardではなく、次の注意遷移を一画面で完結させます。

```text
ObservationBatch
      ↓
有限Brief proposal（最大3件、根拠必須）
      ↓
自然言語triageをCodex ACPへ送る
      ↓
Triage proposal
      ↓ 本人が確認
Decision / CuriosityCapture / ReturnAnchor
```

Return Threadは画面左に常時残り、Agent sessionやDesktopを再起動してもSQLiteから復元します。

## Runtime boundaries

| Boundary | Owner | Contract |
|---|---|---|
| Observation ingress | `openbrief ingest` | versioned JSON、冪等batch ID、source coverage |
| Local authority | `openbrief-store` | Observation、proposal、本人確定状態を別tableで保存 |
| Agent control | `openbriefd` + `openbrief-agent` | 公式ACP SDK、明示path、read-only mode、stream、cancel、timeout |
| Agent action | `openbrief mcp serve` | `brief_propose`、`triage_propose`だけ |
| Confirmation | `openbriefd` command | 選択要素だけを一transactionで本人状態へ確定 |
| UI | React / Flutter controller | Brief、Return Thread、conversation、readiness、proposal確認 |

ACPとMCPを同じものとして扱いません。ACPはDesktopとstateful Agentのsession lifecycle、MCPはAgentからOpenBriefへ提案を書き込む狭いaction planeです。

## Target deployment: daemon版とApp版

現行MVPは、同じheadless binaryを二つの起動形態で使う。「serverless daemon」はcloud serverを要求せず、本人の端末またはx870で完結するlocal-first daemonを意味する。

```text
openbrief-core
  ├─ openbriefd       SQLite、collector、VLM、ACP runtimeのowner
  ├─ openbrief CLI    local daemon client
  ├─ OpenBrief App    Tauri desktop client
  └─ Flutter App      authenticated remote client
```

### Daemon版

Linuxでは`systemd --user`が`openbriefd`を所有する。GUIを起動していなくても、Activity Recall、scheduled producer、VLM変換、Brief生成、Agent sessionを継続できる。CLIとTauriはUnix domain socketへ接続する。Windowsへ展開する場合はnamed pipeとOS service相当を別adapterとして追加する。

### App版

system serviceを設定したくない利用者向けに、Tauriが同じ`openbriefd` binaryをmanaged childとして起動する。App終了時はgraceful shutdownし、配下のACP、MCP、Agent subprocessをprocess tree単位で回収する。既にsystemd版daemonが動いている場合は新しいchildを起動せず、そのinstanceへ接続する。

### 共通のownership

- SQLiteを開いて書き込むauthorityは`openbriefd`だけにする
- ACP adapter、session、permission broker、cancel、process cleanupも`openbriefd`が所有する
- CLI、Tauri、Flutterはraw SQLiteやraw ACP JSON-RPCへ接続しない
- local clientはUnix socket / named pipe、FlutterはTailscale上の認証済みHTTP / WebSocketを使う
- remote APIはsnapshot、Agent turn、proposal確認と、それらのdomain eventだけを公開する
- FlutterへCodex credentialやDesktopのmaster keyを複製せず、失効可能なdevice固有credentialを発行する

現行Linux実装はcontrol socketをsingle-instance境界にする。接続可能なsocketがあればTauriはそのdaemonへattachし、接続不能なstale socketだけを削除してmanaged childを起動する。データschemaとapplication serviceは共通にし、起動ownerとtransportだけをadapterとして分ける。

Unix socket上のdomain command / event contract、`openbriefd`へのstore・Agent ownership移行、bearer token付きremote transport、Flutter companionまでは実装済み。自動pairingとdevice token失効管理は次段階とする。local RPCをそのままインターネットへ公開しない。

Rustのwire型は`openbrief-protocol`、Unix socket clientは`openbrief-client`へ分離する。Dart側も`packages/openbrief_client`がHTTP / WebSocketとwire modelを所有し、Flutter Appはsecure storage、画面状態、表示だけを所有する。

Desktopは`Status.control_protocol_version`を起動時に検証する。systemd等が管理する古いdaemonへ接続した場合は、そのprocessを勝手に停止せず、serviceの再起動またはupgradeを求める明示的なerrorで終了する。

## Data and privacy

- Agentへ渡すObservationは最新batchの先頭100件まで
- serialized snapshotは64 KiBまで
- source本文はuntrusted evidenceであり、命令として実行しない
- Agent executableはconfigの絶対pathだけを使う
- unknown tool、external write、runtime download、Agent/provider fallbackは提供しない
- LM Studioは将来のscreen observation producer専用であり、この推論経路へ入れない

Agentがremote modelを使う場合、bounded snapshotは端末外へ送られる可能性があります。OpenBriefはこれをlocal処理とは表示しません。

## Desktop port

Reactは`DesktopPort`だけへ依存します。

- `FixtureDesktopPort`: browserでdesignとreducer flowをdeterministicに確認する
- `TauriDesktopPort`: typed Tauri eventとcommandを実データへ接続する

AI SDKのchat stateやtransport abstractionは使いません。状態はplain reducer、streamはACP event、永続化はRust application serviceが所有します。

## Operational setup

DesktopはBuzzのmanaged-agent設計から静的runtime catalogだけを採用する。catalogはprovider ID、表示名、同梱path、引数、version policy、認証方法の優先hint、OpenBrief MCPの付与可否を持ち、共通のresolve / probe / start / authenticate flowへ変換する。ACP transport自体にはprovider traitを追加しない。選択providerが変わった場合は稼働中runtimeを停止してから新しいentryを起動する。

現在のcatalogはOpenBrief package内の固定版Codex ACPとPi ACPを登録する。`agent.provider = "codex"`が既定で、`agent.executable_path`は別buildを検証する場合だけのadvanced overrideである。dynamic harness、PATH探索、自動install、provider fallbackは導入しない。

Pi providerはACP v1の会話、stream、tool表示、cancelを共通runtimeで扱う。ただし`pi-acp` v0.0.33はACPで受け取ったMCP serverをPiへ転送せず、terminal authもACP `authenticate` requestでは実行しない。このため`openbrief_mcp`を無効にし、Pi側のmodel provider設定を事前条件とする。構造化proposalとDesktopからのterminal loginは、upstream capabilityまたは明示的な別設計が整うまで対応範囲に含めない。

Nix packageには`openbrief`、`openbriefd`、`openbrief-desktop`が同じ`bin` directoryへ入ります。Desktopは同じdirectoryのdaemonをmanaged childとして、daemonは同じdirectoryのCLIをMCP subprocessとして起動します。

## Scheduled producer

Gmail / Slackの定期収集はDesktopやCodex ACPの責務ではありません。Hermes Agentのcronが`gog` CLIやSlack CLIをread-onlyで実行し、結果を正規化して次へ渡します。

```text
Hermes cron
  ├─ gog CLI
  └─ Slack CLI
        ↓ normalize
  ObservationBatch JSON
        ↓
  openbrief ingest
        ↓
  OpenBrief SQLite
```

HermesはSQLiteを直接書き換えず、version付きingress contractを使います。外部sourceへの返信、削除、status変更などのwrite actionは、この収集経路へ混ぜません。

## Not in this MVP

- Gmail / Slack credentialとsource API adapter
- Hermes cron job自体の自動設定
- OpenClaw / Claude runtime
- screen captureとLM Studio VLM
- Observation全文検索MCP
- Agentからの返信、calendar、Slack status等のexternal write
- 複数同時session、process pool、heartbeat
