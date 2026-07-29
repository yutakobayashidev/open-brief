# 24. Bundled Coast CLIのclient contract

## 結論

bundled `coast`は、local appへnewline-delimited JSON-RPC 2.0を送る薄いUnix socket clientである。一つのsocketを再利用し、接続断では一度だけ再接続して同じrequestを再送する。

retrieval funnelとhuman / JSONの二つのprojectionは参考になる。一方、partial write、response ID照合、response size上限、timezone付きtimestamp、schema version、capability negotiationがなく、OpenBriefが互換実装としてコピーすべきwire contractではない。

本章は独立実装の設計材料としてclient behaviorを要約する。Coast互換clientやprivate RPCの再実装は目的にしない。

## Transport

socket pathは次である。

```text
$HOME/Library/Application Support/inc.attention.rem/cli.sock
```

確認できたclient flow:

```text
AF_UNIX / SOCK_STREAMへconnect
  → compact JSON-RPC request + LFをsend
  → LFまで64 KiB単位でrecv
  → JSON decode
  → errorを先に処理
  → resultを返す
```

requestは概ね次のfieldを持つ。

```json
{"jsonrpc":"2.0","method":"...","params":{},"id":1}
```

request IDはprocess内で0から始まり、最初のrequestが1になる。socketはcommand実行中に再利用される。

主要function:

| Address | 役割 |
|---|---|
| `0x10009dfc4` | JSON-RPC requestの組立とresult / error処理 |
| `0x10009e7a0` | socket path生成 |
| `0x10009e8e8` | client初期化 |
| `0x10009e9f0` | connection確立 |
| `0x10009f1cc` | send / receive |
| `0x10007964c` | RPC errorのCLI表示 |

## Failure contract

`EPIPE`または`ECONNRESET`ではsocketを閉じ、一度だけ再接続してrequestを再送する。send / receive timeoutは100秒である。

確認できた弱点:

- request全体に対して`send()`を一回だけ呼び、partial write loopがない
- response IDとrequest IDを照合しない
- responseの`jsonrpc` versionを検証しない
- LFまでのtotal response sizeに上限がない
- EOF時はLFがなくても蓄積済みresponseを返す
- cancellation、rate limit、payload limit、schema negotiationがない
- read timeoutの100秒は対話CLIとして長い

Darwinでは`EAGAIN` / `EWOULDBLOCK`の値35をtimeoutとして扱う。

JSON-RPC errorでは`-32602`と`-32601`を個別に表示する。invalid JSON、result / error不在等はgeneric invalid responseになる。

exit code:

| 状況 | Code |
|---|---:|
| success、help、version、引数なしhelp | 0 |
| runtime / operational failure | 1 |
| syntax / validation failure | 64 |

## Time parsing

point timestampはlocal timezoneのnaive文字列として解釈される。確認できたformat:

```text
yyyy-MM-dd'T'HH:mm:ss
yyyy-MM-dd'T'HH:mm
yyyy-MM-dd HH:mm:ss
yyyy-MM-dd HH:mm
```

`Z`やUTC offset付きISO 8601は受けず、`TimeZone.current`へ依存する。date-onlyはpoint timestampとして受理しない。

rangeはday、`start|end`、since、before、それらの組み合わせを持つ。

Activity RecallではDST、timezone変更、remote shellを扱うため、このcontractを採用しない。OpenBriefのmachine outputはoffset付きRFC 3339、human inputは明示されたlocal timezoneで解決し、曖昧・存在しないlocal timeをerrorにする。

## Commandとoutput

確認したread surface:

```text
applications / domains
usage / sessions
query fts / sample / cover
query frame / ocrboxes / image / axtree
grab-screen
```

human outputはcompact表示、JSONはpretty printされたsorted keyである。FTS、sample、coverのOCR本文はJSON modeだけで返す。JSONにschema versionはなく、errorはhuman-readable textだけである。

主なdefault:

| Parameter | Default / lower bound |
|---|---|
| top limit | 10 |
| FTS limit | 50 |
| session gap | 5分、minimum 1分 |
| sample minimum segment | 10秒 |
| cover difference threshold | 0.2 |
| cover time threshold | 10秒 |

cover / sampleはstart boundを要求する。一方、limit上限、batch response上限、hard range capは確認できない。helpにあるcoverの30分推奨は強制ではない。

coverは時系列順に最初のframeを選び、以後は直近5枚の選択済みOCRに対するTF-IDF cosine distanceとtime thresholdの両方を満たすframeを追加する。

`grab-screen`は`screen.capture` RPCを呼び、JPEGを`/tmp/coast-cli/`へ保存する。大部分がread-onlyのCLIでも、このcommandだけはlocal filesystemへ書く。

## OpenBriefで採るcontract

OpenBriefはCoast protocolとの互換性を持たせず、同じ「狭いlocal CLIから段階的に読む」考え方だけを採る。

```text
transport:
  Unix socketはmode 0600
  bounded request / response
  write_all + read_until with size cap
  short deadlineとcancellation
  request ID、protocol version、response kindを検証

time:
  machine I/Oはoffset付きRFC 3339
  dateとlocal timeにはtimezone IDを付ける
  query rangeにhard cap

output:
  --jsonはschema_versionを持つ
  stdoutはprimary data、stderrはdiagnostic
  structured error codeとexit codeを安定化
  queried range、gap、truncation、evidence levelを返す
```

command defaultはcommand内に散らさず、application layerのtyped requestへ一度だけ正規化する。Agent integrationも同じAPIを使い、socket methodを直接公開しない。

## 最小fixture

1. 1 byteずつのpartial write / readでも一requestを復元できる
2. response ID不一致、version不一致、二重responseを拒否する
3. size cap超過をconnection closeしてerrorにする
4. deadline後のlate responseを次requestのresponseとして扱わない
5. DST fold / gap、offset付きtimestamp、timezone変更を固定する
6. JSON successとJSON errorの両方に`schema_version`を要求する
7. 1日超のrangeとlimit上限超過をclient / serverの両方で拒否する
