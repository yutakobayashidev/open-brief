# 07. AI Agent連携

## 結論

Attention / CoastのAgent連携は、一つのmagicな統合ではなく、少なくとも次の三層に分かれている。

```text
Coast recording database
        │
        ▼
Unix socket JSON-RPC ── Coast CLI ── structured query
                         │
                         ▼
                  Agent用coast skill
                         │
        ┌────────────────┼────────────────┐
        ▼                ▼                ▼
   Claude / Codex     Cursor          OpenClaw

Coast UIの /agent
        └─ promptをdesktop appのdeep linkまたはterminal CLIへ渡す
```

Agentが過去の活動を自律的に参照できる主因は、専用MCP serverではなく、**検索可能なCLIと、そのCLIの使い方を教えるskill**の組み合わせだと判断できる。`/agent`は逆方向に、Coast内の依頼を外部Agentへ渡す別機能である。

## 1. CLI bridge

### 確認

binaryには次の型と文字列がある。

- `CLIBridgeService`
- `CLIBridgeRouter`
- `CLIRPCHandler`
- `NewlineFrameDecoder`
- `CLI bridge listening on %s`
- `RPC request: %s`
- `Dispatching RPC: %s`
- `Invalid request: missing method`
- `Method not found`

逆コンパイルで確認したflowは次である。

```text
Coast CLI
  → Application Support配下のcli.sock
  → SwiftNIO Unix-domain socket
  → LFまでを一つのframeとしてdecode
  → NSJSONSerialization
  → id? / method / params? を抽出
  → method router
  → JSON-RPC 2.0 resultまたはerror + LF
```

`CLIBridgeService`は1 threadのevent loop groupとserver bootstrapを作り、起動前に既存socket pathを除去する。overrideがない場合はApplication Support、bundle identifier、`cli.sock`からpathを組み立てる。buildごとの正確な親directory名は実機確認が必要である。

request objectは`id`を任意、`method`を必須String、`params`を任意dictionaryとして読む。responseは`jsonrpc: "2.0"`と同じ`id`を持つ。Parse error、Invalid Request、Method not found、Internal errorには標準JSON-RPC error codeを使う。

入口からrouter dispatchまでにtoken、API key、Bearer、handshake、peer credentialのcheckはない。socket作成pathでも明示的な`chmod 0600`は確認できなかった。したがってapplication layerの認証はなく、親directoryとsocketのOS permissionへ依存する設計と判断する。

次は未確認である。

- runtimeで作られるsocketと親directoryの実mode
- 同一UIDのpeer checkが別layerに存在する可能性
- frame size、rate limit、同時接続上限

### Query surface

型、method文字列、error pathから、少なくとも次のcapabilityが存在する。

| capability | binary evidence |
|---|---|
| application一覧 | `list.applications`、`CLIApplicationInfo` |
| domain一覧・resolve | `CLIDomainInfo`、`Failed to get/resolve domains` |
| application/domain別利用時間 | `CLIBridgeUsageStats`、`topApplications`、`topDomains` |
| session集計 | `UsageSessionsResponse`、`usageSessions` |
| OCR全文検索 | `coast query fts ...`、`FTSSearchService` |
| frame解決 | timestampまたはframe IDを一つ指定するvalidation |
| OCR・image取得 | `Failed to query OCR/image` |
| 時間範囲sample | segment sample、cover sampleと件数上限 |
| Accessibility取得 | frame AX tree、attribute audit、live AX result型 |
| 現在画面のcapture | screen capture / image encode error path |
| Agent skill設定 | `skill.installAll` |

router switchで確認したmethodは次である。

```text
usage.time
usage.byApplication
usage.byDomain
usage.sessions
list.applications
list.domains
resolve.domains
query.fts
query.frame
query.ocr
query.image
query.cover
query.axtree
query.axattrs
screen.capture
screen.axtree
app.skillPath
skill.status
skill.statusAll
skill.install
skill.installAll
```

segment sampleとskill uninstallにもhandlerがあるが、最終method名は確定できていない。

時間範囲queryには必須parameter、segment/frame件数上限、短い範囲を使うよう促すerrorがある。Agentへ無制限な全履歴dumpを返すのではなく、queryをboundedにする設計である。

## 2. Agent skillの配布

### 確認

`AgentSkillManager`は少なくともClaude、Codex、Cursor、OpenClawをtargetとして扱う。

bundled resourceとinstall状態には次のevidenceがある。

- `.app/contents/resources/skill`
- `skill/coast-cli-skill`
- `/coast-cli-skill`
- `isAgentInstalled`
- `isSkillInstalled`
- `installForAllDetectedAgents(force:)`
- `Installed skill for %s`
- `Skill already installed for %s`

Claude、Codex、Cursor向けにはskill pathのsymlinkを管理していることが強く推定できる。既存pathが通常fileの場合や、別のskillを指すsymlinkの場合は上書き・削除しない。

OpenClawだけは`.openclaw/openclaw.json`を読み、`skills.load.extraDirs`へbundled skill directoryを追加する専用pathがある。既存の別Coast entryがある場合はskipし、uninstall時は自分が追加したentryだけを外す意図がlogから読める。

この方式はAgentごとのplugin APIへ深く結合するのではなく、**同じskill内容を各Agentが発見できる場所へ公開するadapter**である。

### CLI installation

Coast CLI自体はapp bundleからuser環境へsymlinkされる構造を持つ。

- `Bundled CLI not found, cannot create symlink`
- `~/.local/bin`をPATHへ追加する案内
- shell profileがなければ`.zshrc`を作るpath

Agent skillは、実行可能な`coast` commandがPATHにあることを前提にできる。skillとCLIを分けているため、Agent側のintegrationは薄い。

## 3. `/agent` routing

### 対象検出

`AgentAppLauncher`とagent install detectionは次を個別に検出する。

- Claude CLI / Claude desktop app
- Codex CLI / Codex desktop app
- Cursor CLI / Cursor desktop app
- OpenClaw

CLIは`which`相当のprobe、desktop appはbundle IDで判定する。`first installed agent`を使うdefaultと、agent selection preferenceがある。

今回のbinaryにはDevin固有文字列がない。ユーザー提供のDevin CLI利用談は、genericなCoast CLIをDevinが直接発見・実行した事例か、別build・別設定による可能性があり、専用Devin integrationの証拠にはしない。

### Prompt delivery

Claude、Codex、Cursorには二つのdelivery pathがある。

1. desktop appのdeep linkでpromptをpre-fillする
2. Terminalで`claude`、`codex`、`cursor-agent`を起動する

desktop pathでは、deep link生成、app起動、front/raise、composer確認、Return key eventによるauto-sendに分かれる。auto-sendはAccessibility trustがない、appが起動していない、composerを確認できない場合にskipするlogがある。

Terminal起動に失敗した場合はpromptをclipboardへcopyするfallbackがある。これはdata lossを避けるUXだが、clipboardへ機密contextを置くprivacy trade-offも持つ。

## 4. 「Agentが自然に記憶を使う」仕組み

binary内のsample promptは次の形である。

```text
What apps did I use the most this week? Use coast.
Summarize my work sessions today. Use coast.
Find the design review I was reading. Use coast.
```

対応するstructured CLI例も埋め込まれている。

```console
coast usage top-applications --tr "since:..."
coast usage sessions --tr "since:..."
coast query fts "design review"
```

したがってAgent連携の中心は、model contextへ全履歴を常時注入することではない。

```text
userまたはAgentの質問
  → skillがCoast CLIのquery方法を選ぶ
  → localでbounded query
  → 必要な結果だけAgent contextへ戻す
```

これはprompt肥大化を避け、必要時だけpersonal memoryをpullするretrieval modelである。ユーザー提供の「Claude CodeやDevin CLIが自然にCoast CLIをqueryした」という利用談とも整合するが、その自律判断の頻度や再現性はbinaryだけでは検証できない。

## 5. OpenBriefへの採用判断

### 採用する

1. 人間とAgentで同じread modelを使う
2. JSON出力を持つ小さなCLIを唯一のAgent data planeにする
3. queryは時間範囲、件数、payload sizeをboundedにする
4. Agent別integrationはCLIの使い方を教えるskill adapterに限定する
5. raw DB pathをAgentへ渡さず、安定したprojectionだけ返す

```console
openbrief today --json
openbrief around 14:00 --json
openbrief search "design review" --since 7d --limit 20 --json
```

### MVPでは採用しない

- 常駐RPC daemon
- MCP server
- desktop app deep link
- Accessibilityによるauto-send
- promptのclipboard fallback
- Agentからのdelete、retention変更、capture制御
- Agentごとに異なる検索backend

CLI-onlyのOpenBriefでは、processがread-only connectionでSQLiteを直接queryする方が単純で、local listenerの認証・lifecycle・protocol互換性を背負わない。複数processの調停やTauri常駐processが必要になった時だけ、Unix domain socketを別crateとして追加する。その場合は親directory `0700`、socket `0600`、同一UID peer確認、payload上限をAttentionより明示的に実装する。

### Skillの最小形

OpenBriefのskillは、巨大なAgent orchestrationを実装せず、次だけを教える。

- どんな質問で`today`、`around`、`search`を使うか
- JSON fieldの意味
- `gap`とmodel summaryを事実と混同しないこと
- secretや除外期間を推測しないこと
- dataがなければ「不明」と答えること
- write/delete commandは存在しないこと

Agent連携の価値は専用protocolの多さではなく、**信頼できる小さなread interfaceをAgentが必要時に発見できること**にある。
