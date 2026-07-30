# Pi ACP source reference

## Snapshot

- Repository: [`svkozak/pi-acp`](https://github.com/svkozak/pi-acp)
- Version: `0.0.33`
- Commit: [`1bfcb394088ed879db8fd936b570bb626017f878`](https://github.com/svkozak/pi-acp/tree/1bfcb394088ed879db8fd936b570bb626017f878)
- License: MIT

このsnapshotをOpenBriefのPi provider実装基準とする。minor breaking changeが予告されているため、mainへ自動追従しない。

## Runtime contract

`pi-acp`はACP JSON-RPC 2.0のNDJSONをstdioで受け、内部で`pi --mode rpc --no-themes`を起動する。ACP protocolはv1、session cwdは絶対path必須である。

- entrypoint: [`src/index.ts`](https://github.com/svkozak/pi-acp/blob/1bfcb394088ed879db8fd936b570bb626017f878/src/index.ts)
- ACP initialize/session: [`src/acp/agent.ts`](https://github.com/svkozak/pi-acp/blob/1bfcb394088ed879db8fd936b570bb626017f878/src/acp/agent.ts)
- Pi child process: [`src/pi-rpc/process.ts`](https://github.com/svkozak/pi-acp/blob/1bfcb394088ed879db8fd936b570bb626017f878/src/pi-rpc/process.ts)

OpenBriefは固定版adapterとflake lock内のPi v0.81.1をNix packageへ含め、`PI_ACP_PI_COMMAND`を絶対pathへ固定する。`npx latest`、PATH探索、runtime downloadは使わない。

## Adopt

- 静的runtime catalogへ`provider = "pi"`を追加する
- ACP initialize、session、prompt stream、tool event、cancel、process cleanupは既存の共通runtimeを使う
- `pi-acp`は`--version`を実装しないため、CLI probeを行わずACP v1 handshakeへ互換性確認を委ねる
- Piのmodel providerとcredentialはPi自身の設定を使う

## Explicit limitations

[`README`のLimitations](https://github.com/svkozak/pi-acp/blob/1bfcb394088ed879db8fd936b570bb626017f878/README.md#limitations)どおり、ACP `session/new`のMCP serverは保存されるだけでPiへ転送されない。したがってOpenBrief MCPを付与せず、`brief_propose`と`triage_propose`はPi providerで利用できない。

認証method `pi_terminal_login`は`pi-acp --terminal-login`の対話terminal起動を要求する一方、ACP `authenticate` handler自体はno-opである。OpenBriefは認証済みPiを前提とし、Desktopの認証buttonで設定できるとは表示しない。

Piはhost userの権限でfile、process、networkへ直接アクセスし、ACP filesystem / terminal delegationを使わない。OpenBriefのpermission UIがPiの操作をsandbox化するとは主張しない。

## Revisit when

- `pi-acp`がACP MCP serverをPiへ実際にforwardする
- terminal authの標準ACP contractがOpenBriefのRust SDKで利用可能になる
- `pi-acp`が安定したversion probeまたは互換性policyを公開する
- Pi providerで構造化proposalを実証する必要が生じる
