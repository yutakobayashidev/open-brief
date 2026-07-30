# Mobile companionとremote API

## Outcome

Flutter companionは録画閲覧器でも汎用chat clientでもない。`openbriefd`が作った有限Briefを手元で確認し、自然言語でtriageを相談し、本人確認後にReturn Threadを保存する。

## Boundary

```text
Flutter UI
  └─ Dart openbrief_client
       ├─ HTTPS request
       └─ WSS domain events
              ↓
       openbriefd remote adapter
              ↓
   application service / ACP / SQLite
```

- `openbrief-protocol`: Rustのtransport非依存wire型
- `openbrief-client`: CLI / Tauri用Unix socket client
- `packages/openbrief_client`: Flutter等から再利用するDart HTTP / WebSocket client
- `apps/mobile`: secure storage、connection lifecycle、UI
- `openbriefd`: authorization、domain operation、Agent process、SQLiteのauthority

FlutterはACP process、Codex credential、SQLiteを所有しない。

remote APIはACPやCodex app-serverのproxyではない。Agentとのstdio ACPは`openbriefd`内で終端し、MobileへはOpenBriefのdomain commandとeventだけを公開する。transport選定の理由は[ADR 0004](../adr/0004-separate-acp-local-and-remote-transports.md)に固定する。

## API v1

`GET /health`だけが認証不要。他のendpointは`Authorization: Bearer <device-token>`を要求する。

| Method | Path | Meaning |
|---|---|---|
| `GET` | `/v1/snapshot` | Brief、Return Thread、pending proposal、Agent status、event cursor |
| `PUT` | `/v1/agent-session` | singleton Agent sessionをreadyにする |
| `POST` | `/v1/turns` | 有限長の自然言語triageを開始する |
| `POST` | `/v1/proposals/{id}/confirmations` | 本人判断としてproposalを確定する |
| `GET` | `/v1/events?after=N` | `openbrief.events.v1` subprotocol必須のWebSocket event stream |

Agent session開始とproposal確認は冪等に扱う。event sequenceは再接続用cursorであり、永続的な監査logではない。WebSocketはserver-to-client専用で、commandやraw ACP JSON-RPCを受け付けない。再接続後はsnapshotを正本として読み直す。

## Security

remote APIは既定で無効、bind既定値は`127.0.0.1:43117`。token fileは絶対pathかつowner以外が読めないmodeを要求する。daemon自身はTLSを終端しないため、loopbackで待ち受け、Tailscale Serve等のprivate network ingressからHTTPS / WSSを提供する。

初期MVPは一つのpre-shared device tokenをsecure storageへ保存する。QR pairing、端末ごとのtoken発行・失効、certificate pinningは未実装であり、公開Internetへ露出しない。
