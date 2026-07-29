# 15. Startup、single-instance、crash recovery

## 結論

Attentionの常駐captureは、application launchと同時に無条件で録画を始めない。

```text
single-instance
    ↓
critical startup tasks
    ↓
DB接続・schema / migration recovery
    ↓
permission readiness
    ↓
Rewind migration gate
    ↓
recording
    ↓
background startup tasks
```

この順序、recording intentとruntime stateの分離、fileとDBを照合する起動時recoveryはOpenBrief recorder daemonへ直接使える。一方、macOS app relocationとLogin ItemはLinuxへ移植しない。

## Single-instance handoff

`CoastSingleInstanceGuard`はlock競合時に即終了するだけではない。

1. 既存sibling PIDをactivateする。
2. 50ms間隔でlock取得を再試行する。
3. 再度surface requestを送る。
4. 100ms間隔の延長待機を行う。
5. 既存instanceがなおlockを持てば新processを終了する。

主処理は`FUN_10063b3e0`、直接duplicate確認は`FUN_10063bc78`である。終了中の旧processから新processへlockを引き継ぐ短い猶予がある。

lock file path、lock primitive、総wait時間、PID reuse対策、surface request transportは未確認である。

OpenBriefはdaemon起動の最初にsingle-instance lockを取る。二重起動時は既存daemonのUnix socketへ`status`を送り、CLIへ既存instanceの状態を返す。stale判定ではPIDだけでなくprocess start timeまたはboot IDも照合する。

## Startup task orchestration

`OnceTasksService`はcritical taskとbackground taskを分ける。

- critical start: `FUN_100305210`
- critical completion: `FUN_1003055f4`
- background start: `FUN_100305c6c`
- background completion: `FUN_100305fb4`
- named task cache / skip: `FUN_1003114dc`
- failure record: `FUN_1003118fc`
- critical failure propagation: `FUN_100003e44`

taskは名前ごとにstatusを持ち、完了済みの重複実行をskipする。確認できたtask keyには次がある。

- `check_rewind_migration`
- `initial_database_setup`
- `migrate_from_legacy_local_store`

完全なtask DAG、並列度、timeoutは未確認である。

## Recording gate

recorder start `FUN_10017469c`は少なくとも次を確認する。

1. screen recording permission
2. DB connection
3. Rewind migrationがpending / in-progressでないこと

permissionが後から変わる場合も共通reconcileへ戻る。DB未接続やread-only connectionではrecordingを始めず、migration完了後にretryする。

重要なのは、録画したいという`intent`を捨てず、dependencyがreadyになるまでruntime startをdeferする点である。

```text
recording_intent = on
runtime_state = waiting_for_database

database connected
    ↓ reconcile
runtime_state = waiting_for_permission

permission granted
    ↓ reconcile
runtime_state = recording
```

AX observerのような補助dependencyはstartup timeout後もdeferできる。core captureを止めるhard dependencyと、後から接続できるdegraded dependencyを分けている。

## Crash and interrupted-work recovery

### In-flight capture

`FUN_10029e824`のwatchdogはstaleなin-memory reservationをdropし、timestamp順write queueの詰まりを解除する。reservationをDBへ永続化してprocess crash後にreplayする証拠はない。

OpenBriefでもreservationは永続化しない。再起動後は全reservationを捨て、last committed timestampから再開する。

### Interrupted migration

`FUN_1002da8a0`はpartial artifactsとpartial videos directoryを削除し、migration stateを整理する。これはresumeよりrollback cleanupを選ぶpathである。

### Video archive

archive recoveryはDB stateとfile existenceを照合する。

- scan entry: `FUN_1003c7d30`
- archive fileがある: `FUN_1003d9458`でarchivedとしてfinalize
- archive fileがない: `FUN_1003d97cc`でactiveへ戻す

一方向に「成功したはず」と仮定せず、外部artifactの存在を見て完了またはrollbackを選ぶ。

OpenBriefのfile protocolは次でよい。

```text
write tmp
  → fsync
  → atomic rename
  → DB commit
```

起動時にpending rowとfileの存在を照合し、finalize可能なら完了、なければpendingへ戻す。

### 未確認のrecovery

- FFmpeg temporary fileのcrash後処理
- segment reservationの再構築
- upload queueの再起動resume
- signed URLのidempotency

`VideoUploadStatus.pendingUrlCount`は存在するが、durable upload recoveryを証明するcallsiteは見つからなかった。将来OpenBriefへuploadを加える場合は、durable queue、idempotency key、server ACK receiptを独自に要求する。

## Wake、power-off

- wake entry: `handleSystemDidWake: @ 0x100be12fc`
- power-off entry: `AppDelegate::handleWillPowerOff @ 0x100013bf4`

wakeはMainActor async taskを起動するが、具体的なreconcile先は未確認である。power-offは非同期shutdown taskを開始し、一度だけimmediate terminationを許すstateを持つ。

OpenBriefはwake時にsource、permission、display、idle stateを再probeし、shutdown時には新規captureを止めて進行中commitだけを有限時間待つ。

## Relocationとautostart

macOS版はApp Translocation、DMG、一時directoryからの起動を検出し、Applicationsへcopy後にinstalled copyをrelaunchする。Login Itemは`SMAppService`へ委譲し、遅い非同期結果でUIを巻き戻さないgeneration管理を持つ。

これはdesktop product deliveryとしては有用だが、Linux CLI MVPでは不要である。OpenBriefはsystemd user serviceに任せ、CLIでは次だけを提供する。

```text
openbrief service install
openbrief service status
openbrief service uninstall
```

## OpenBrief recorder daemon案

```text
Locking
  → Recovering
  → OpeningDatabase
  → Migrating
  → ProbingSources
  → Ready | ReadyDegraded
  → Recording | Paused(reason)
```

最低限の原則:

1. single-instance lockを最初に取る。
2. intentとruntime stateを分離する。
3. dependency changeごとに一つのresolverでreconcileする。
4. hard dependencyとdegraded dependencyを分ける。
5. capture laneはboundedにし、stale reservationをwatchdogで捨てる。
6. crash後はDBとfileを照合し、推測で完了扱いしない。
7. startup failureとcapture gapにmachine-readable reasonを付ける。

## 未確認

- single-instance lockの実装詳細
- startup taskの完全なDAGとtimeout
- wake taskの最終処理
- upload queueのdurable recovery
- relocation / Login Item timeout
