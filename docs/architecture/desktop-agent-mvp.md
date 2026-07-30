# Desktop Agent MVP

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
| Agent control | `openbrief-agent` | 公式ACP SDK、明示path、read-only mode、stream、cancel、timeout |
| Agent action | `openbrief mcp serve` | `brief_propose`、`triage_propose`だけ |
| Confirmation | Tauri command | 選択要素だけを一transactionで本人状態へ確定 |
| UI | React reducer | Brief、Return Thread、conversation、readiness、proposal確認 |

ACPとMCPを同じものとして扱いません。ACPはDesktopとstateful Agentのsession lifecycle、MCPはAgentからOpenBriefへ提案を書き込む狭いaction planeです。

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

現在のcatalogはOpenBrief package内の固定版Codex ACPだけを登録する。`agent.provider = "codex"`が既定で、`agent.executable_path`は別buildを検証する場合だけのadvanced overrideである。dynamic harness、PATH探索、自動install、provider fallbackは導入しない。

Nix packageには`openbrief`と`openbrief-desktop`が同じ`bin` directoryへ入ります。Desktopは同じdirectoryのCLIをMCP subprocessとして起動します。

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
