# ADR 0004: ACP、local control、remote APIのtransportを分離する

## Status

- Accepted

## Decision Drivers

- Agent実装をCodex app-server固有APIへ結合せず、ACP runtimeとして交換可能に保つ
- local daemon操作、Agentとの双方向session、mobileの不安定なnetworkでは必要な性質が異なる
- FlutterへAgent credential、raw ACP session、filesystem / terminal capabilityを公開しない
- protocol統一のための汎用router、schema変換、再接続処理を増やさない

## Context

OpenBriefには三つの通信境界がある。

1. `openbriefd`からACP Agent subprocess
2. CLI / Tauriから同一端末の`openbriefd`
3. Flutter companionからtailnet内の`openbriefd`

ACP v1の中心は`initialize`、必要時の`authenticate`、`session/new`、`session/prompt`、`session/update`、`session/cancel`である。OpenBriefは公式Rust SDKを使い、Agent subprocessのstdio上でACP JSON-RPCを終端している。

Codex app-serverはrich Codex client向けにthread、turn、approval、auth等をJSON-RPCとして提供する。これは参考になるが、OpenBriefが所有するBrief、本人確認、Return Threadとはauthorityもdomainも異なる。

BuzzもAgent harnessからAgentへはstdio ACPを使う一方、MobileはACPへ接続せず、Relay上のdomain messageとobserver eventを使う。

## Options Considered

- 全境界をJSON-RPC 2.0へ統一する: request ID、notification、双方向requestを共通化できるが、local one-shot操作とmobile domain APIへ不要なsession semanticsを持ち込む
- Codex app-serverをremote backendとして使う: Codex UI機能は得られるが、Codex固有schemaとcredentialがOpenBriefのpublic contractになる
- raw ACPをWebSocketでFlutterへ中継する: 実装は直結できるが、permission、filesystem、terminal、session recoveryをMobileへ漏らす
- 境界ごとに最小transportを選ぶ: 重複は少し残るが、authorityとfailure modelを明確にできる

## Decision

- Agent境界だけをACP v1とする。`openbriefd`がstdio JSON-RPC、initialize、authentication、session、stream、cancel、permission、process cleanupを所有する
- 対応するAgent protocolはACPだけとし、Codex app-server、独自Codex JSON-RPC、provider固有session APIは実装しない
- local controlはmode `0600`のUnix socket上で、一接続一request / responseのnewline-delimited typed JSONを使う。multiplex、server request、notificationが必要になるまでJSON-RPC envelopeを追加しない
- remote commandはversioned HTTPS APIを使う。resource取得とidempotentな状態変更をHTTP statusとtyped bodyで表す
- remote eventは`openbrief.events.v1` WebSocket subprotocol上のserver-to-client domain eventだけとする。FlutterはWebSocketからcommandやraw ACP messageを送らない
- snapshotをremote stateの正本、event journalをforeground更新用のbounded ephemeral streamとする。再接続時はsnapshotを再取得してから新しいcursorで購読する
- TLSと到達制御はTailscale Serve等のprivate ingressへ委任し、`openbriefd`は既定でremote無効・loopback bindとする

## Consequences

- Positive: ACP adapterとMobile APIを独立して変更でき、Agent種別やCodex内部schemaがFlutter contractへ漏れない
- Positive: local CLIはrequest IDや常時接続なしで単純なまま保てる
- Positive: mobile reconnect時にAgent transcriptの完全replayを要求せず、OpenBriefの確定状態を復元できる
- Negative: Rust local clientとDart remote clientは同じtransport実装を共有しない
- Negative: WebSocketが切れている間の途中経過は失われる。確定Brief、proposal、Return Threadはsnapshotから復元する
- Follow-up: background完了通知が必要になった時はpush notificationを別adapterとして追加し、WebSocket常駐を前提にしない
- Follow-up: ACPのremote transport案がstableになっても、OpenBrief domain APIを置き換える根拠にはしない。daemonからremote ACP Agentを起動する要件が出た時だけ内部Agent境界として評価する

## Adoption and Exceptions

- `openbrief-agent`以外のcrateはACP wire型を直接扱わない
- `apps/mobile`とDart client packageへACP method名、Agent credential、filesystem / terminal capabilityを追加しない
- remote WebSocket endpointは`openbrief.events.v1` subprotocolを必須にする
- 新しいremote mutationはHTTP APIへ追加し、対応するdomain commandと本人確認境界を文書化する
- 例外は新ADRで、必要な双方向性、再接続、security boundary、既存方式では不可能な理由を示す

## References

- [ACP v1 overview](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/protocol/v1/overview.mdx)
- [ACP transports](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/protocol/transports.mdx)
- [Codex app-server README at the reviewed revision](https://github.com/openai/codex/blob/6256a7ccc7948231befc33d7d61b369041e6eb16/codex-rs/app-server/README.md)
- [Buzz ACP harness at the reviewed revision](https://github.com/block/buzz/blob/61b96c9828d1dd54106b570d87a54edbc92bb9c4/crates/buzz-acp/src/acp.rs)
