# Attention binary観測記録

## 記録方針

- addressは今回Ghidraへloadされたbinary内のvirtual addressであり、別versionでは一致しない。
- decompiler出力や第三者codeは記録しない。
- 長いSQLはschemaと判定条件だけを要約する。
- **確認**と、そこから導いた**推定**を分ける。

## Platformとmodule

| Address / evidence | 観測 | 確度 |
|---|---|---|
| `0x100001c60` | ScreenCaptureKit framework path | 確認 |
| `0x100001e58` | Vision framework path | 確認 |
| `0x100001598` | ApplicationServices framework path | 確認 |
| `0x100d8fdf0` | `AttentionUtils.ScreenCaptureService` Swift type | 確認 |
| `0x100d902a0` | `AttentionUtils.CaptureThrottler` Swift type | 確認 |
| `0x100d8ffa0` | `AttentionUtils.AccessibilityObserver` Swift type | 確認 |
| `0x100d900f0` | `AttentionUtils.LiveAccessibilityTree` Swift type | 確認 |
| `0x100d8fd30` | `AttentionApplications.ExcludedAppsService` | 確認 |
| `0x100d8fd70` | `AttentionApplications.ExcludedDomainsService` | 確認 |

## Capture

| Address | 文字列またはsymbolの要約 | 導けること |
|---|---|---|
| `0x100f34330` | interval指定、max concurrent 3でrecording開始 | timer captureとbounded concurrency |
| `0x100f34120` | capture slot取得、active/maxをlog | slot管理 |
| `0x100f34150` | queue fullでframe drop | overload時にqueueを増やさない |
| `0x100f34450` | session終了後に返ったframeを破棄 | stale session guard |
| `0x100f34520` | frameをHEICへencode | recent frameは静止画保存 |
| `0x100f347b0` | focused window除外でskip | capture前privacy判定 |
| `0x100f34900` | focused app除外でskip | capture前privacy判定 |
| `0x100f34930` | allowed/excluded window数 | ScreenCaptureKit filter |
| `0x100f34850` | focused browser windowを読めない場合にskip | browser contextのfail-closed path |
| `0x100f34ed0` | timed pause期限後にresume | pause deadline |
| `0x100f35460` | inactivity後にframeをinactive扱い | inactive marking |
| `0x100f354b0` | inactivity後にrecordingをpause | configurable inactivity policy |

逆コンパイルで確認した主なfunction:

- `CaptureThrottler` acquire: `0x10016e7b8`
- `CaptureThrottler` release: `0x10016e988`
- `ScreenCaptureService`中心capture: `0x1000988f8`
- `ScreenshotRecorder` start: `0x100174c1c`
- frame collection: `0x100177fc4`
- capture stage計測集約: `0x10017ca34`

## OCR

| Address | 文字列の要約 | 導けること |
|---|---|---|
| `0x100f33e40` | persistent OCR request初期化 | Vision request再利用 |
| `0x100f33e80` | previous frameなし/size不一致でfull OCR | full path条件 |
| `0x100f33f30` | previous frameからdifferential OCR | 差分path |
| `0x100f34090` | region crop失敗時にfull OCR | safe fallback |
| `0x100f34040` | previous/new OCR segmentsをmerge | 差分結果の統合 |
| `0x100f33ec0` | preprocess/OCR/postprocess時間 | stage別計測 |

逆コンパイルで確認した主なfunction:

- OCR gate acquire: `0x1001610a4`
- OCR gate release: `0x1001615b0`
- persistent request初期化: `0x1001619c0`
- OCR入口: `0x100162088`
- differential/full分岐: `0x100162500`
- OCR segment merge: `0x100164174`
- Vision OCR本体: `0x100166388`

## Accessibility

| Address | 観測 | 導けること |
|---|---|---|
| `0x100ea0850` | `partial_publish_interval_nodes` | node数でpartial publish |
| `0x1012550f9` | progressive publish node interval metadata | incremental tree build |
| `0x101255134` | progressive publish time interval metadata | time budget |
| `0x100ea0d00` | Dia Accessibility waker | app別AX activation |
| `0x100ea0d20` | Electron Accessibility waker | Electron対応 |
| `0x100ea0f70` | Gecko Accessibility waker | Firefox系対応 |
| `0x100ea1150` | WebKit Accessibility waker | WebKit対応 |
| `0x100f33250` | 除外appのAX observationをskip | screenshotと同じprivacy policy |
| `0x100eb5370` | `ax_snapshot` schema | modeとpartial flagを保存 |
| `0x100eb5320` | hash keyの`ax_node` | content-addressed node |
| `0x100eb50e0` | parent/child hashの`ax_node_edge` | treeをgraphとして分離 |

逆コンパイルで確認した主なfunctionと定数:

- AX専用thread作成: `0x1000f2aa8`
- AX run loopへのblock投入: `0x1000f2914`
- observer start/世代更新: `0x1000fa9f0`
- InputDirtyMonitor初期化: `0x1000f6f48`
- dirty coalesce: 0.05秒
- scroll時のparent climb上限: 10
- continuous drain worker: `0x1001301f8`
- cache drift probe: `0x100132570`
- full rebuild planner: `0x10013b228`
- full rebuild cooldown: 2秒

`SelectionCaptureService`はAX selected textを取得し、最後の値を30秒でexpireする。中心functionは`0x1000a3328`、期限確認は`0x1000a4100`。

## Frame ordering

| Address | 文字列の要約 | 導けること |
|---|---|---|
| `0x100f373a0` | in-flight capture登録 | capture開始を予約 |
| `0x100f373e0` | in-flight capture cancel | reservation lifecycle |
| `0x100f37530` | reservationなしのstale capture reject | late result拒否 |
| `0x100f37680` | earlier in-flight captureを待つ | timestamp順commit |
| `0x100f37710` | write queue overflowでdrop | bounded write queue |
| `0x100f37760` | stale reservation watchdog | deadlock回避 |
| `0x100f37460` | SQLITE_FULL時rollbackとimage cleanup | DB/file整合性 |

## Privacy、timeline、disk

| Address | 観測 | 導けること |
|---|---|---|
| `0x10018d210` | recording exclusion変更handler | runtime policy reload |
| `0x10018d424` | domain exclusion変更handler | domain policy reload |
| `0x100f33250` | excluded appのAX observationをskip | 共通privacy gate |
| `0x1008e2878` | timeline range fetch開始時にgeneration取得 | async query世代管理 |
| `0x1008e2cdc` | result適用前にgeneration再検証 | superseded result拒否 |
| `0x10014fa78` | DiskSpaceMonitor開始 | 二重起動を避けた周期監視 |
| `0x100152f58` | critical disk path | fail-closed recording pause |
| `0x100153a48` | disk recovery path | 条件を再検証してpause維持/解除 |

## Core schema

| Address | SQL |
|---|---|
| `0x100eb6e90` | `application` table |
| `0x100eb6f90` | `domain` table |
| `0x100eb7040` | `video` table |
| `0x100eb7100` | `segment(start_frame_id, application, domain, url)` |
| `0x100eb7290` | `frame(timestamp, video/index, image_path, OCR text, title, segment)` |
| `0x100eb74c0` | OCR bounding box table |
| `0x100eb7630` | window bounds/layer/z-order table |
| `0x100eb58f0` | frame contentと同期するFTS5 table |

## Segment

`0x100ebfca0`付近のmigration SQLは、`LAG()`で直前frameのapplication、domain、URLと比較し、いずれかが変化したframeを`segment.start_frame_id`として挿入する。

`0x100eb6690`と`0x100eb6a10`付近には、application/domain/URLが直前と同じ連続segmentを統合・削除するSQLがある。

## Compactionとretention

| Address | 観測 | 導けること |
|---|---|---|
| `0x100f3b9d0` | FFmpeg stdin encoding session開始 | image-to-video compaction |
| `0x100ec52f0` | frameへvideo ID/indexを設定 | compact後の参照 |
| `0x100ec53d0` | frame.image_pathをNULL化 | static image cleanup |
| `0x100f3eed0` | imageがなければvideo extraction | timeline read fallback |
| `0x100f3bd30` | archived/purged/frame数 | retention action |
| `0x100f3a150` | integrity circuit breaker trip | destructive process停止 |
| `0x100f3c450` | video resolutionを下げてarchive | re-encode retention |

## Product-use evidence

ユーザー提供の利用談には次が含まれる。

- Claude Code / Devin CLIがCoast CLIをpersonal memoryとしてquery
- Zoom presentationをrecordingのframeから抽出しPDFへ再構成
- form入力、refreshで消えたdraft、閲覧済みweb contentのsystem of record
- Attention cloudをorganization-level insightへ利用

これはbinary evidenceではなく利用者によるtestimonialである。独立再現、頻度、成功率、enterprise privacyは未確認として扱う。

## Agent integration

| Address | 観測 | 導けること |
|---|---|---|
| `0x100d53496` | `CLIBridgeRouter` | CLI requestのmethod routing |
| `0x100d53610` | `CLIBridgeService` | app側のCLI service |
| `0x100d53670` | `NewlineFrameDecoder` | messageをnewlineで区切る |
| `0x100d53684` | `CLIRPCHandler` | request/response handler |
| `0x10068a4cc` | 1 thread event loop、server bootstrap、filesystem path bind | Unix-domain socket server |
| `0x100689890` | Application Supportと`cli.sock`からpathを構築 | default socket path |
| `0x10068b4e4` | decoderとRPC handlerをchannelへ追加 | connection pipeline |
| `0x100690480` | LFを探して一行ずつ切り出す | newline-delimited framing |
| `0x10068d818` | JSONから`id`、`method`、`params`を読む | request envelopeとdispatch |
| `0x100690744` | JSON-RPC error responseを構築 | standard error envelope |
| `0x100691128` | JSON-RPC result responseを構築 | standard success envelope |
| `0x100690be0` | responseへLFを追加してwrite | response framing |
| `0x100675880` | method文字列を比較するrouter switch | RPC allowlist |
| `0x100f3e430` | CLI bridgeがaddressをlisten | local listener lifecycle |
| `0x100f3e3f2` | RPC request log | bridgeがrequestを受信 |
| `0x100ecc730` | method欠落を拒否 | method-based envelope |
| `0x100ecc0a0` | method not found | allowlisted router |
| `0x100ecc060` | `list.applications` | structured application query |
| `0x100ecc080` | `skill.installAll` | setup mutationもRPC methodとして存在 |
| `0x100d2e700` | `CLIBridgeListQueries` | CLI query response model |
| `0x100d2e7a0` | `CLIBridgeUsageStats` | usage projection |
| `0x100d2ed80` | frame AX attributes result | Agent/CLIからAX情報へ到達可能 |
| `0x100d2ee60` | frame AX tree result | frame単位AX query |
| `0x100d2eec0` | live AX tree result | live AX query surface |
| `0x100d53400` | `AgentSkillManager` | Agent別skill installation |
| `0x100ecc040` | `skill/coast-cli-skill` | app bundle内skill resource |
| `0x100f3e020` | 別pathを指すskillをskip | 既存Agent設定を上書きしない |
| `0x100f3df50` | OpenClaw `skills.load.extraDirs`へ追加 | OpenClaw専用adapter |
| `0x100e9e1b0`–`0x100e9e270` | Claude/Codex/Cursor CLI・app、OpenClawのinstall event | 対象Agentの検出 |
| `0x100f32a90` | Agent app deep linkをopen | desktop prompt routing |
| `0x100f32b10` | AX trustなしでauto-sendをskip | auto-send permission gate |
| `0x100f32be0` | app未起動でauto-sendをskip | target lifecycle check |
| `0x100f3b350` | terminal launch失敗時にclipboardへcopy | delivery fallback |

request parseからrouter dispatchまでにtoken、API key、Bearer、handshake、peer credential checkはない。socket作成pathにも明示的な`chmod 0600`は確認できなかった。runtimeのfilesystem mode、別layerのpeer check、rate limitは未確認とする。

## 未確認

- exact product version、build number、binary hash
- default capture interval
- CaptureThrottlerの全判定式
- AX notificationの完全な購読list
- input dirtyからcaptureまでの正確な同期順
- screen lock、sleep/wake、multi-monitor変更時の全挙動
- telemetryへ送られるfield
- backend data flow
- runtimeでのfeature flag状態
- CLI socketと親directoryのruntime mode、別layerのpeer check、完全なRPC schema
- Agent skillの全文とAgent側の自律query判断
