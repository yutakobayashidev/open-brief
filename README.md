# OpenBrief

「何を見ていたか」だけで終わらず、いま守ること、有限に探索すること、元の作業へ戻る糸を残すlocal-firstなAttention Handoffです。認知科学・HCI研究、既存製品の解析、OSS調査も同じリポジトリに残しています。

現在は次の二つを実装しています。

- Linux / Wayland / niri向けmetadata-only Context Recall: foreground app IDと時刻だけをSQLiteへ保存する
- Attention Handoff MVP: Observationを有限Briefへ変え、Codex ACPとの自然言語triageを本人確認後にだけDecision / Curiosity / Return Anchorへ確定する。Tauri DesktopとFlutter companionから同じdaemonへ接続できる

window title、PID、画面、音声、Agent transcriptは保存しません。x870上のLM Studioは将来の画面VLM用であり、Brief生成はCodex等のAgentへ委任します。

## Try it

```console
nix develop
cargo test --workspace
cargo build --release

# foregroundで安全に試す
./target/release/openbriefd

# 別terminalから確認する
./target/release/openbrief status
./target/release/openbrief recent
./target/release/openbrief today
./target/release/openbrief around 14:00

# ObservationBatchを投入する
./target/release/openbrief ingest tests/fixtures/gc-04-observation-batch.json

# Desktopを起動する
./target/release/openbrief-desktop

# Flutter companionを確認する
cd apps/mobile
flutter run
```

継続利用する場合は、release binaryを固定した場所へ置いてから`openbrief enable`を実行する。systemd user serviceが`openbriefd`を起動し、collector、SQLite書き込み、ACP runtimeを所有する。

```console
openbrief enable
openbrief pause --for 30m
openbrief resume
openbrief delete --today
openbrief disable
```

configは`${XDG_CONFIG_HOME:-~/.config}/openbrief/config.toml`、DBは`${XDG_DATA_HOME:-~/.local/share}/openbrief/openbrief.sqlite3`へ置く。Desktopは既存の`openbriefd`へ接続し、未起動なら同じ`bin` directoryのdaemonをmanaged childとして起動する。Desktopが起動したdaemonだけを終了時に停止し、systemd管理のdaemonは停止しない。

DesktopとCLIはUnix socketで`openbriefd`へ接続する。Desktopは起動時にcontrol protocol versionを確認し、不一致ならserviceの再起動またはupgradeを要求する。Flutter companionはTailscale Serve等でTLSを終端したHTTP / WebSocket APIへ接続する。remote APIは既定で無効かつ平文HTTPなので、loopback以外へ直接bindしない。

```console
install -m 600 /dev/null ~/.config/openbrief/device-token
openssl rand -hex 32 > ~/.config/openbrief/device-token
```

```toml
[remote]
enabled = true
bind = "127.0.0.1:43117"
token_file = "/home/yuta/.config/openbrief/device-token"
```

外部からは`tailscale serve --bg https+insecure://127.0.0.1:43117`等でtailnet内のHTTPSへ公開し、そのURLとdevice tokenをFlutterへ入力する。自動pairingとtoken失効UIはまだ対象外です。

interactive AgentはDesktopの静的ACP runtime catalogから選択する。現在の組み込みproviderは`codex`です。別buildを明示的に試す場合だけ、advanced overrideとして絶対pathを設定する。

```toml
[agent]
provider = "codex"
executable_path = "/absolute/path/to/codex-acp"
```

既定denylistは1Password、Signal、Discord。`delete`はTTY確認が必要で、非対話では`--force --no-input`を両方要求する。

DesktopからAgentへ渡すのは、最新Observation最大100件を含む64 KiB以下のbounded snapshotです。MCPは`brief_propose`と`triage_propose`だけを公開し、Agentは本人のDecisionやReturn Anchorを直接確定できません。Codexが利用するmodel/providerへの送信は`codex-acp`側の設定に従うため、機密Observationを投入する前に確認してください。

## Rust workspace

```text
openbrief-desktop      Tauri command/event adapter、openbriefd managed-child
    └─ React UI        有限Brief、Return Thread、選択中のAgent sidecar

openbrief-cli          ingest、Context Recall、hidden MCP stdio server
openbriefd             collector、SQLite authority、ACP runtime、event journal
    └─ openbrief-app   Attention service、query、daemon、systemd
       ├─ openbrief-agent       公式ACP SDK、process lifecycle、stream/cancel
       ├─ openbrief-client      Unix socket local client
       ├─ openbrief-core         Observation / Proposal / Decision domain
       ├─ openbrief-protocol     transport非依存のcommand / event / API wire型
       ├─ openbrief-source-niri  niri IPC adapter
       └─ openbrief-store        SQLite local authority

openbrief_mobile       Flutter Attention companion
    └─ openbrief_client          再利用可能なDart HTTP / WebSocket client
```

Reactはshadcnのcompositionと視覚patternだけを参考にし、Vercel AI SDK、`useChat`、`@shadcn/helpers`は使いません。`openbriefd`がAgent processとpermission境界を所有します。

主な検証は`cargo test --workspace`、`cargo clippy --workspace --all-targets --all-features -- -D warnings`、`pnpm --dir apps/desktop test`、`pnpm --dir apps/desktop build`、`cd apps/mobile && flutter test`です。GC-02はActivity Recall回帰、GC-04はObservation ingressとDesktop handoff用fixtureです。

## Nix flake

flakeはLinux向けのpackage、app、overlay、Home Manager moduleを公開する。

```console
nix build .#
nix run .# -- --help
```

利用可能なoutput:

```text
packages.<system>.default
packages.<system>.openbrief
packages.<system>.openbrief-codex-acp
apps.<system>.default
apps.<system>.desktop
overlays.default
homeManagerModules.default
homeManagerModules.openbrief
homeModules.default
```

Home Managerではconfigとsystemd user serviceを一緒に宣言する。

```nix
{
  inputs.openbrief.url = "github:yutakobayashidev/open-brief";

  outputs =
    {
      home-manager,
      nixpkgs,
      openbrief,
      ...
    }:
    {
      homeConfigurations.yuta = home-manager.lib.homeManagerConfiguration {
        pkgs = nixpkgs.legacyPackages.x86_64-linux;
        modules = [
          openbrief.homeManagerModules.default
          {
            services.openbrief = {
              enable = true;
              settings = {
                retention_days = 7;
                capture.excluded_apps = [
                  "1password"
                  "signal"
                  "vesktop"
                ];
              };
            };
          }
        ];
      };
    };
}
```

Home Manager管理時は`openbrief enable / disable`を使わず、`home-manager switch`へservice lifecycleを一本化する。既存の`~/.config/openbrief/config.toml`がある場合は、必要な値を`services.openbrief.settings`へ移してから既存fileを退避または削除する。Home Managerは未管理fileを勝手に上書きしない。

serviceは`graphical-session.target`から起動する。niriは`niri-session`または`niri --session`で開始し、`NIRI_SOCKET`をsystemd user managerへimportしておく。raw `niri`起動時のsocket探索や固定path fallbackは提供しない。

## Architecture Decisions

- [Design memo: OpenBrief as an Attention Control Plane](docs/architecture/attention-control-plane.md)
- [Desktop Agent MVP](docs/architecture/desktop-agent-mvp.md)
- [Mobile companionとremote API](docs/architecture/mobile-companion.md)
- [ADR一覧](docs/adr/README.md)
- [ADR 0001: Local-firstなデータ境界とModel Gateway](docs/adr/0001-adopt-local-first-data-and-model-boundaries.md)
- [ADR 0002: Attention SignalとSlack Status Output](docs/adr/0002-adopt-attention-signals-and-slack-status-output.md)
- [ADR 0003: Proposal-only ACP Agent boundary](docs/adr/0003-adopt-proposal-only-acp-agent-boundary.md)
- [ADR 0004: ACP・local・remote transportの分離](docs/adr/0004-separate-acp-local-and-remote-transports.md)

## Tiimo調査レポート

- [調査概要と目次](docs/reverse-engineering/tiimo/README.md)
- [独自実装ブループリント](docs/reverse-engineering/tiimo/05-reimplementation-blueprint.md)

## Attention macOS静的解析

- [調査概要と目次](docs/reverse-engineering/attention/README.md)
- [Captureとcontext取得](docs/reverse-engineering/attention/03-capture-and-context-pipeline.md)
- [OpenBriefへの採用判断](docs/reverse-engineering/attention/05-openbrief-adoption.md)
- [AI Agent連携](docs/reverse-engineering/attention/07-agent-integration.md)
- [追加バイナリ解析マップ](docs/reverse-engineering/attention/08-further-analysis-map.md)
- [Browser privacy解析](docs/reverse-engineering/attention/09-browser-privacy-path.md)
- [Usageとsession semantics](docs/reverse-engineering/attention/10-usage-and-session-semantics.md)
- [Sync・upload・airgap解析](docs/reverse-engineering/attention/11-sync-upload-airgap.md)
- [Searchとretrieval pipeline](docs/reverse-engineering/attention/12-search-retrieval-pipeline.md)
- [Time state・inactivity・timezone](docs/reverse-engineering/attention/13-time-state-and-inactivity.md)
- [Evidence・artifact recovery](docs/reverse-engineering/attention/14-evidence-and-artifact-recovery.md)
- [Startup・single-instance・recovery](docs/reverse-engineering/attention/15-startup-and-recovery.md)
- [Invocation・selection・overlay](docs/reverse-engineering/attention/16-invocation-selection-and-overlay.md)
- [Rewind import・video salvage](docs/reverse-engineering/attention/17-rewind-import-and-salvage.md)
- [Telemetry・airgap・onboarding](docs/reverse-engineering/attention/18-delivery-telemetry-and-onboarding.md)
- [Retention・delete完全性](docs/reverse-engineering/attention/19-retention-delete-integrity.md)
- [Capture trigger state machine](docs/reverse-engineering/attention/20-capture-trigger-state-machine.md)
- [Privacy transition race](docs/reverse-engineering/attention/21-privacy-transition-races.md)
- [Agent skill・bundle監査](docs/reverse-engineering/attention/22-agent-skill-and-bundle-audit.md)
- [Production DB暗号化境界](docs/reverse-engineering/attention/23-production-database-encryption.md)
- [Coast CLI client contract](docs/reverse-engineering/attention/24-coast-cli-client-contract.md)
- [Manual capture privacy境界](docs/reverse-engineering/attention/25-manual-capture-privacy-boundary.md)

## Attention Triage研究

- [研究概要と目次](docs/research/attention-triage/README.md)
- [Gmail＋RSSゴールデンケース](docs/research/attention-triage/03-golden-case.md)
- [TiimoとOpenBriefの比較](docs/research/attention-triage/05-tiimo-comparison.md)
- [構想の客観評価](docs/research/attention-triage/06-objective-assessment.md)
- [ADHD向けContext ResumptionとOracleレビュー](docs/research/attention-triage/07-adhd-context-resumption-oracle-review.md)
- [awesome-adhd横断レポート](docs/research/attention-triage/08-awesome-adhd-cross-report-synthesis.md)
- [Resume CueとWindow Transitionを比較するMVP](docs/research/attention-triage/09-window-transition-mvp-reset.md)
- [入力不要のActivity Recall Timeline MVP](docs/research/attention-triage/10-activity-recall-timeline-mvp.md)
- [qwen-audio-agent調査とaudio採用判断](docs/research/attention-triage/11-qwen-audio-agent-assessment.md)
- [Capture substrateとAgent consumer分離](docs/research/attention-triage/12-capture-substrate-and-agent-consumers.md)
- [GC-01実装fixture](fixtures/golden-cases/gc-01-gmail-rss-return.json)
- [GC-02 Activity Recall fixture](fixtures/golden-cases/gc-02-activity-recall-timeline.json)
- [GC-03 Activity Recall fail-closed fixture](fixtures/golden-cases/gc-03-activity-recall-fail-closed.json)
- [GC-04 Desktop Attention Handoff fixture](tests/fixtures/gc-04-observation-batch.json)
- [評価プロトコル](docs/research/attention-triage/04-study-protocol.md)

## OSS implementation references

- [参照方針と目次](docs/implementation-references/README.md)
- [Screenpipe source reference](docs/implementation-references/01-screenpipe-source-reference.md)
- [Entire CLI source reference](docs/implementation-references/02-entire-cli-source-reference.md)
- [Buzz source reference](docs/implementation-references/03-buzz-source-reference.md)

解析対象APKは `apks/com.tiimo.androidappreactnative/` に置かれています。APKや復元コードを配布・転載することを目的としていません。
