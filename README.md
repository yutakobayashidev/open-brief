# OpenBrief

入力を要求せず「いつ何をしていたか」を思い出せる、local-firstなContext Recall CLIです。認知科学・HCI研究、既存製品の解析、OSS調査も同じリポジトリに残しています。

現在はLinux / Wayland / niri向けのmetadata-only R0を実装しています。foreground app IDと時刻だけを7日間SQLiteへ保存し、window title、PID、画面、音声、agent transcriptは取得しません。画面captureとx870上のLM Studio連携は、R0を実際に使って価値とprivacyを確認した後のR1です。

## Try the R0

```console
nix develop
cargo test --workspace
cargo build --release

# foregroundで安全に試す
./target/release/openbrief watch

# 別terminalから確認する
./target/release/openbrief status
./target/release/openbrief recent
./target/release/openbrief today
./target/release/openbrief around 14:00
```

継続利用する場合は、release binaryを固定した場所へ置いてから`openbrief enable`を実行する。systemd user serviceが同じbinaryのhidden `watch` commandを起動する。

```console
openbrief enable
openbrief pause --for 30m
openbrief resume
openbrief delete --today
openbrief disable
```

configは`${XDG_CONFIG_HOME:-~/.config}/openbrief/config.toml`、DBは`${XDG_DATA_HOME:-~/.local/share}/openbrief/openbrief.sqlite3`へ置く。既定denylistは1Password、Signal、Discord。`delete`はTTY確認が必要で、非対話では`--force --no-input`を両方要求する。

## Rust workspace

```text
openbrief-cli          CLI、human / JSON output
    └─ openbrief-app   query、collector、systemd、privacy control
       ├─ openbrief-core         stateと15分projection
       ├─ openbrief-source-niri  niri IPC adapter
       └─ openbrief-store        SQLite authority
```

R0の主な検証は`cargo test --workspace`と`cargo clippy --workspace --all-targets --all-features -- -D warnings`。GC-02 fixtureを、focus event → SQLite → 15分ActivitySliceの統合テストとして実行する。

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
apps.<system>.default
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

- [ADR一覧](docs/adr/README.md)
- [ADR 0001: Local-firstなデータ境界とModel Gateway](docs/adr/0001-adopt-local-first-data-and-model-boundaries.md)
- [ADR 0002: Attention SignalとSlack Status Output](docs/adr/0002-adopt-attention-signals-and-slack-status-output.md)

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
- [評価プロトコル](docs/research/attention-triage/04-study-protocol.md)

## OSS implementation references

- [参照方針と目次](docs/implementation-references/README.md)
- [Screenpipe source reference](docs/implementation-references/01-screenpipe-source-reference.md)
- [Entire CLI source reference](docs/implementation-references/02-entire-cli-source-reference.md)
- [Buzz source reference](docs/implementation-references/03-buzz-source-reference.md)

解析対象APKは `apks/com.tiimo.androidappreactnative/` に置かれています。APKや復元コードを配布・転載することを目的としていません。
