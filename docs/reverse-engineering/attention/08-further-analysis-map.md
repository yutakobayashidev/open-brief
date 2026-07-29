# 08. 追加バイナリ解析マップ

## 目的

これまでの解析はcapture、OCR、Accessibility、storage、retention、Agent連携へ集中した。Attention / Coastのbinaryには、それ以外にもOpenBriefの設計判断へ使えるsurfaceが残っている。

本章は「機能名を列挙する」ためではなく、次の追加解析で何を確定でき、OpenBriefのどの判断が変わり得るかを優先度付きで整理する。

## 優先順位

| 優先度 | 領域 | OpenBriefで答えたい問い |
|---|---|---|
| P0 | Browser URLとprivate browsing | URLを取れない・取るべきでない時にどうfail closedするか |
| P0 | cloud sync、upload、airgap | local-firstとoff-device dataをどこで分離するか |
| P0 | search rankingとdedup | 「見たものを思い出す」検索で何を残し、何を畳むか |
| P0 | usage/session時間の算出 | 時間盲向けtimelineの時刻をどう事実として計算するか |
| P1 | hotkey、overlay、selection | goal入力なしでも低摩擦に呼び出せる入口は何か |
| P1 | lifecycleとfailure recovery | 常駐captureがpermission変更、二重起動、移行中にどう止まるか |
| P1 | Rewind migrationとsalvage | 他製品dataから安全にimportする一般原則は何か |
| P1 | Evidence / artifact recovery | summary-onlyを越える価値を最小raw evidenceで出せるか |
| P2 | onboardingとguided tour | ambient toolの価値を最初の数分でどう体験させるか |
| P2 | telemetry、edition、update | feature flagや計測がprivacy境界を迂回しないか |

## P0-1. Browser URLとprivate browsing

### Binary evidence

- `BrowserURLService`
- `BrowserURLReadHealthMonitor`
- `BrowserURLReadState` / `BrowserURLReadDecision`
- `browserUrlUnreadable`
- `Set/Loaded/Updated exclude private browsing`
- 「Automation accessでincognito stateを直接読み、なければ推定する」という説明
- browser URLが読めないfocused windowではframeをskipするcapture path

### 追加解析で確定できそうなこと

- browserごとのURL取得adapter
- Apple Events / Accessibility / window titleの優先順
- private browsingを直接判定できるbrowserと推定しかできないbrowser
- URL read failureのhealth state、cooldown、再試行
- domain exclusionとprivate browsing exclusionの評価順
- browser permissionがruntimeで失われた時の挙動

### OpenBriefへの価値

niri版ではbrowser URLをMVP必須にしないが、将来browser adapterを追加する時のfail-closed contractに直結する。特に「URLが読めなければtitleから推測して記録する」のか「unknown gapにする」のかを決められる。

### 限界

macOSのAutomation permissionとLinux browser integrationは同じではない。制御flowは借りてもOS実装は再利用できない。

## P0-2. Cloud sync、video upload、airgap

### Binary evidence

- `DatabaseSyncRequest` / `SyncResponse` / `SyncState`
- `DatabaseSyncStatus` / `AxTreeSyncPhase`
- `BackendConnectionManager` / `BackendWebSocketService`
- production databaseをread-onlyで開くsync pipeline
- `VideoUploadStatus`
- `settings.storage.deleteLocalVideosAfterUpload`
- 「upload後にlocal videoを削除し、再downloadまでplaceholder表示」
- `AirgapModeStore`
- airgapは再起動後に有効となり、telemetry、update check、remote faviconを抑止する説明
- signed-out startup policy

### 追加解析で確定できそうなこと

- frame、segment、OCR、AX node、videoのsync順序
- upload consentとrecording consentが別か
- queue、retry、backpressure、resume token
- upload完了を何で確認してlocal copyを削除するか
- signed-out時にcapture、query、uploadのどれが継続するか
- airgapが起動時だけsnapshotされる理由と、無効化されるnetwork subsystem
- debug DBをproductionへuploadしないguard

### OpenBriefへの価値

OpenBriefは現在cloud syncを採用しないが、x870のLM Studioはoff-device egressである。Attentionの境界を追うことで、**capture consent、model送信、長期sync、削除**を同じtoggleへ混ぜない設計を確認できる。

また、将来cross-device syncを検討する時に、local deletionをserver acknowledgmentだけで実行してよいか、integrity checkとrecoveryをどう置くかの材料になる。

### 限界

client binaryだけではserver側retention、authorization、tenant isolation、enterprise analyticsを証明できない。endpoint protocolの完全復元も本調査の目的にしない。

## P0-3. Search ranking、streaming、dedup

### Binary evidence

- FTS5、BM25、highlight
- streaming search
- title dedup
- application/domain dedup
- TF-IDF similarity dedup
- local OCR dedup
- Jaccard bigram / character similarity、sparse cosine similarity
- query validation、filter-only streaming
- stageごとの件数とlatency log

### 追加解析で確定できそうなこと

- raw FTS hitから最終resultまでのstage順
- title、app/domain、time、OCR、similarityのrankingへの寄与
- 同じpageを何度も見た時に一件へ畳むwindow
- search query grammar、quote、parenthesis、app/domain/time filter
- TF-IDF cacheの構築、更新、利用不能時のdegradation
- streaming chunkとcancellationの境界

### OpenBriefへの価値

Activity Recallの`search`で、同じterminal/browser stateを大量に返さず、かといって重要な再訪を消さない設計に使える。ただしMVPでこのpipelineを再実装せず、synthetic fixtureで単純なtime + text searchの不足を確認してから採用する。

## P0-4. Usage/session時間の算出

### Binary evidence

- `CLIBridgeUsageStats`
- `UsageSessionsResponse`
- `topApplications`
- `topDomains`
- `usageSessions(startMs:endMs:gapSeconds:...)`
- SQL上のframe timestamp、segment boundary、inactive flag

### 追加解析で確定できそうなこと

- frame intervalから利用時間へ変換する式
- 長いcapture gapをactive timeへ数える上限
- inactivity、pause、excluded、missing frameの扱い
- sessionを分割するgap threshold
- app/domainが重なる場合の帰属
- timezone、日跨ぎ、retention cutoff

### OpenBriefへの価値

時間盲支援ではVLM summaryより先に「いつ」の正しさが重要である。ここを追うと、観測gapを活動時間として水増ししない集計contractを作れる。Attentionの値をそのまま使わず、OpenBrief fixtureの期待値へ一般原則を反映する。

## P1-1. Hotkey、overlay、selection

> 解析済み。結果は[16 Invocation、selection、overlay](16-invocation-selection-and-overlay.md)を参照。

### Binary evidence

- `HotKeyCenter`
- `SearchShortcut`
- double Command / Shift-double Command
- search / timeline hotkey policy
- hotkey recording中のshortcut suppression
- Codex shortcut conflict detectionと移動
- modalがhotkeyを飲み込まないためのpath
- `SelectionCaptureService`
- menu bar、floating window、overlay presenter

### 追加解析で確定できそうなこと

- search、timeline、Agent routingを開くshortcut state machine
- permission不足時のdegradation
- shortcut conflictの検出とuser choice
- 現在選択中textをquery seedへ使うflow
- overlayをcapture対象から外すrecording gate
- overlayを閉じた後のfocus restoration

### OpenBriefへの価値

goal入力を要求しない方針と相性がよい。CLI MVPでは必須でないが、Tauri段階で「現在地を失わず、選択textまたは時刻をseedに一発で検索する」入口を設計できる。

## P1-2. App lifecycleとfailure recovery

> 解析済み。結果は[15 Startup、single-instance、crash recovery](15-startup-and-recovery.md)を参照。

### Binary evidence

- `CoastSingleInstanceGuard`
- duplicate launch時に既存instanceをactivate
- startup task status
- migration中はrecording開始を延期
- runtime permission revoke時にwizardへ戻る
- app relocation、launch-on-login、auto start
- stale capture、write queue、disk guard

### 追加解析で確定できそうなこと

- startup taskの依存順とfailure policy
- 二重起動時のlock、wait、incumbent activation
- permission、migration、DB接続、recordingのstate reconciliation
- sleep/wake、screen lock、display changeの再初期化
- crash後の一時file、reservation、upload queue recovery

### OpenBriefへの価値

CLI-onlyでも`record` daemonを持つなら、happy pathより先に必要になる。特に「起動したがcaptureできていない」silent failureを避け、理由付きgapへ変換する設計へ使える。

## P1-3. Rewind migrationとvideo salvage

> 解析済み。結果は[17 Rewind import、migration、video salvage](17-rewind-import-and-salvage.md)を参照。

### Binary evidence

- `RewindDatabaseReader`
- `RewindMigrationService`
- `RewindVideoSalvageService`
- encrypted DBとkey要求
- schema compatibility check
- disk space preflight
- cross-volume video判定
- migration中のrecording gate
- temporary clone、decrypted temporary DB
- retry / skip / completed marker

### 追加解析で確定できそうなこと

- source DBを直接変更しないcopy-on-write / clone方針
- schema version判定とunsupported時の停止
- ID、timestamp、OCR、video indexのmapping
- partial migration後のresumeとidempotency
- disk不足、missing video、corrupt frameのsalvage
- migration成功判定とtemporary data cleanup

### OpenBriefへの価値

Screenpipeや他のlifelogからimportする場合のclean-room importer設計に使える。MVPには不要だが、既存historyを捨てずに移行できることは後のadoption barrierを下げる。

## P1-4. Evidenceとartifact recovery

> 解析済み。結果は[14 Evidence、代表frame、artifact recovery](14-evidence-and-artifact-recovery.md)を参照。

### Binary evidence

- `MemoryCardView`
- screenshot containerのloading / failed state
- frame IDからstatic imageまたはvideo frameを取得する`FrameExtractor`
- focused window boundsによるcrop fallback
- timestamp / frame query
- segment sample / cover sample
- Live Text / QR解析

### 追加解析で確定できそうなこと

- memory cardへ選ばれるframeの基準
- representative frame、cover、segment sampleの違い
- active window cropのfallback順
- missing local mediaをcloudから再取得するUI state
- 複数frameからartifactを再構成するために必要な最小metadata

### OpenBriefへの価値

presentation PDF、消えたform draft、過去のerror復元はActivity Recall summaryだけでは実現できない。この領域を追うと、24時間の短期Evidence Storeでどこまで価値を出せるかを別MVPとして定義できる。

## P2. Product delivery系

> telemetry、airgap、update、onboardingの主要境界は解析済み。[18 Telemetry、airgap、update、onboarding](18-delivery-telemetry-and-onboarding.md)を参照。

### Onboardingとguided tour

permission、browse、search、timeline、Agent skill installationまでを段階的に体験させるstate machineを解析できる。価値はactivation設計だが、CLI MVPのcore correctnessより後でよい。

### Telemetry

Sentry、TelemetryDeck、analytics heartbeat、event名は追える。どのcontentが送られるかはruntime trafficなしでは確定できない。OpenBriefでは「何を真似るか」より、content-free metricsのallowlist作成に使う。

### Edition、settings、update

editionごとのsettings catalog、feature surface、Sparkle update channel、blocking update、Codex shortcut conflictを解析できる。製品運用の参考にはなるが、Activity Recallの価値検証へ直接寄与しない。

## 優先しない解析

次はbinaryに存在しても、現段階では時間を使わない。

- SwiftUIのlayout、色、animationの復元
- Sentry、GRDB、SwiftNIO、Sparkle自体の内部実装
- private server endpointや認証回避
- license、account、edition restrictionの回避
- third-party assetの抽出
- exact Coast互換CLI / RPCの実装

## 追加解析の進捗

当初のP0 / P1項目は独立章へ整理した。さらに次を追加解析した。

- [Retention、delete、除外の完全性](19-retention-delete-integrity.md)
- [Capture trigger state machine](20-capture-trigger-state-machine.md)
- [Privacy transition race](21-privacy-transition-races.md)
- [Agent skillとapp bundle監査](22-agent-skill-and-bundle-audit.md)
- [Production databaseの暗号化境界](23-production-database-encryption.md)
- [Bundled Coast CLIのclient contract](24-coast-cli-client-contract.md)
- [Manual captureとhistory readのprivacy境界](25-manual-capture-privacy-boundary.md)

残る高価値項目の多くは静的解析よりmacOS runtime観測が必要である。

1. production DB header、WAL / SHMの平文内容、DB / media / socket permission
2. pause・除外・private browsing中のmanual captureとknown frame ID read
3. sleep / wake、permission revoke、multi-monitor hotplug
4. delete-after-uploadのACKとrestart resume
5. telemetry、update、favicon、backendのactual network traffic

Search rankingの追加最適化とUI復元は、OpenBrief MVPの情報利得が低いため優先しない。
