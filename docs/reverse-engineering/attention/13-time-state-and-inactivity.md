# 13. Time state、inactivity、timezone

## 結論

Attentionは無操作を単なるcapture gapにしない。5秒周期でOS全体の最終入力からinactiveへの遷移を判定し、設定に応じて録画を止めるか、録画を続けてframeへ`is_inactive`を付ける。

一方、sleep、screen lock、display topology changeを独立したactivity eventとして保存する証拠は確認できなかった。timezoneはIANA identifierの変更履歴を別tableへ保存するが、一部の日別集計はquery時の現在timezoneに依存する。

OpenBriefではAttentionより明示的に、UTC timestampを順序軸とし、activity stateとcapture policyを分離する。

## Idle detector

### 確認

- timeout setter: `FUN_100190178 @ 0x100190178`
- monitor start: `FUN_1001902f8 @ 0x1001902f8`
- idle判定: `FUN_100190780 @ 0x100190780`
- poll interval: `5.0`秒
- 入力source: `_CGEventSourceSecondsSinceLastEventType(0, 0xffffffff)`
- DB migration: `v9_add_is_inactive_flag`
- schema変更: `frame.is_inactive INTEGER NOT NULL DEFAULT 0`

判定は概ね次の状態遷移になる。

```text
every 5 seconds
    ↓
seconds since any input event
    ↓
idleSeconds >= configured timeout?
    ├─ no  → active
    └─ yes → inactive
               ├─ pauseOnInactivity = true  → recordingをpause
               └─ pauseOnInactivity = false → recordingを継続しframeをinactive扱い
```

activeへ戻ると共通のrecording state reconcileが呼ばれる。したがって、`wantsToRecord`、実際のrecorder state、inactivityによる一時pauseは別の状態である。

### 設計上の意味

「入力がない」と「画面に価値がない」は同じではない。動画視聴、presentation、build待ち、通話では入力なしでも画面履歴に意味がある。Attentionが二つのpolicyを持つのはこの差を残すためと読める。

Activity Recallでは初期値を`capture_but_mark_idle`とし、query時にidle区間を折り畳む方がよい。記録時に削除すると、離席直前の状態や受動的な閲覧を後から復元できない。

## Sleep、wake、screen lock

### 確認

- `NSWorkspaceDidWakeNotification`を購読する。
- `handleSystemDidWake: @ 0x100be12fc`がMainActor上のasync taskを起動する。
- `NSWorkspaceWillSleepNotification`、screen lock/unlock notification、`CGSessionCopyCurrentDictionary`、IOKit power callbackの利用は確認できなかった。
- power-offは`AppDelegate::handleWillPowerOff @ 0x100013bf4`から非同期shutdown taskを起動する。

### 未確認

wake後のasync taskがrecorder、backend、UIのどれをreconcileするかは、最適化されたSwift binaryから確定できなかった。sleepやlock区間をDBへ独立eventとして保存する証拠もない。

CGEventの最終入力時刻だけに依存すると、sleep、lock、単なる離席を区別できない。OpenBriefではこれらをidleから推測しない。

```text
activity_state =
  active
  | idle
  | sleeping
  | locked
  | unknown
```

Linux adapterは利用可能なOS signalを独立eventへ変換し、wake/unlock時に次を一括でreconcileする。

- capture source
- permission / portal session
- display list
- foreground source
- idle baseline

## Display change

`NSApplicationDidChangeScreenParametersNotification`のhandlerは確認できたが、対象はfloating windowの再配置であり、capture pipelineの再初期化ではない。

capture側は毎回active displayを選び、frameへ`capture_display_x/y/width/height`を保存する。frontmost app window、`NSScreen.main`、最初のdisplayというfallbackを持つが、display topology変更を記録する専用eventは未確認である。

OpenBriefはcapture rectとdisplay IDをframeへ保存し、display追加・削除・scale変更をsource adapterのeventとして扱う。UI windowの位置補正とは同じmoduleにしない。

## Timezone observation

### 確認

`TimezoneObservationService`はpoll loopと`NSSystemTimeZoneDidChangeNotification` listenerを併用する。

- start: `FUN_10031db68 @ 0x10031db68`
- notification handler: `FUN_10031ec24 @ 0x10031ec24`
- record routine: `FUN_10031e32c`
- completion / logging: `FUN_10031e6d4 @ 0x10031e6d4`

schemaは小さい。

```sql
CREATE TABLE timezone (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  identifier TEXT NOT NULL,
  observed_at INTEGER NOT NULL
);

CREATE INDEX idx_timezone_observed_at
ON timezone(observed_at);
```

直前と同じidentifierならinsertをskipする。offsetではなくIANA timezone identifierを保持するため、対象日時のDST offsetをFoundationで再計算できる構造である。

### 注意点

timezone historyを過去frameの表示へ適用する直接的なqueryは未確認である。また、日別集計の一部はSQLiteの`'localtime'` modifierを使う。これは旅行後の再queryで、記録当時と異なるday bucketになる可能性がある。

OpenBriefでは次をcontractにする。

1. `captured_at_utc_ms`を唯一の順序軸にする。
2. `timezone_observation(identifier, observed_at_utc_ms)`を別tableにする。
3. UIで`recorded-local`と`current-local`を区別する。
4. 日別集計は現在timezoneのSQLへ委ねず、UTC range取得後にtimezone履歴を適用する。
5. 完全再現性が必要なら`utc_offset_seconds_at_capture`も保存する。

## MVPへの採用判断

| 項目 | 判断 |
|---|---|
| 5秒周期のidle edge検出 | 採用候補 |
| idle frameのflag | 採用 |
| idle時の既定録画停止 | 不採用。最初は折り畳み |
| sleep / lockをidleから推定 | 不採用 |
| UTC timestamp | 採用 |
| timezone transition table | 旅行を跨ぐ検証時に採用 |
| query時の現在`localtime`による日別集計 | 不採用 |

## 未確認

- wake taskの最終callee
- sleep / lock detectorが別processや動的frameworkに隠れている可能性
- timezone poll interval
- timezone historyをtimeline表示へ適用する実装
- capture recorderのdisplay topology再初期化
