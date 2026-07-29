# 11. Sync、upload、airgap

## 結論

Attentionはoff-device処理を一つのcloud toggleへまとめていない。

```text
Database sync
  ├─ table sync
  └─ AX tree sync

Video pipeline
  ├─ encode queue
  ├─ upload queue
  └─ optional local delete

Control plane
  ├─ authentication startup policy
  ├─ upload consent
  ├─ backend connection/version state
  ├─ debug/production DB guard
  └─ launch-time airgap
```

ただしserver acknowledgment、retry/idempotency、airgapが実際に停止する全serviceはclient静的解析だけでは確定できない。

## Pipeline separation

### Database sync

`DatabaseSyncStatus`は次を持つ。

- paused
- current table
- total synced
- total errors
- last error
- table progress
- blocked reason
- AX tree sync status

table progressにはbackend、local、missing-in-gaps countがある。AX treeは`AxTreeSyncPhase`と専用statusを持ち、work limiterも別に存在する。

### Video upload

`VideoUploadStatus`は次を持つ。

- paused
- uploading
- encode queue size
- upload queue size
- pending URL count
- total uploaded
- total errors
- last error

statusだけからも、encode、upload URL待ち、uploadを分ける三段pipelineが強く示唆される。

DB syncとvideo uploadには共通のpause/resume controlがあるが、状態とwork queueは別である。

## Debug / production DB guard

`0x10020cc80`はproduction `rem.db`をread-only poolとしてsync用に開く。

localhost backendの場合だけproduction poolをskipし、dev poolからsyncできるflagを立てる。別のguardはproduction DBがなければsyncを停止し、debug dataのuploadを避ける。

```text
DB identity不明
  → uploadしない

localhost development backend
  → explicit dev override時だけdev DB

production backend
  → production DB read-only pool
```

OpenBriefが将来syncを持つ場合、pathだけでなくenvironment、schema UUID、database instance IDを起動時に照合する。

## Authentication startup

binaryには次がある。

- `SyncSignedOutStartupPolicy`
- `SyncSignedOutStartupDecision`
- `SyncAuthenticatedStartupSession`
- `startupDecision(isAuthenticated:)`
- `allowsAuthenticatedInitialization`

signed out中にrecordingをpauseするUI文言もある。ただしcaptureに認証を要求するかはedition依存の可能性があり、全buildで一律とは確定できない。

重要なのは、認証完了後にsyncだけをその場で始めるのではなく、startup initialization boundaryを通す点である。

## Upload consent

upload consentはmenu bar controller protocolを介して変更される。UIがupload serviceのflagを直接書き換える構造ではない。

```text
user action
  → consent controller
  → MainActor task
  → pipelineへ変更通知
```

capture consent、Agent query、video upload consentは別の境界として扱うべきである。

## Delete after upload

独立設定:

```text
settings.storage.deleteLocalVideosAfterUpload
```

設定の保存は確認できたが、local file削除が次のどの時点かは確定していない。

- signed URL PUT完了
- backend DB反映
- remote checksum検証
- durable acknowledgment受領

OpenBriefでは次を必須にする。

```text
upload
  → remote ID + checksum + acknowledged_atをdurableに保存
  → 別transaction / 別GC job
  → local copy削除
```

UI toggleから直接fileを削除しない。

## UI visibilityとpipeline pause

overlay表示時はnotificationでvideo uploadをpauseし、非表示時にresumeするpathがある。重いencode/uploadがtimeline UIとresource競合しないためのcontrolと考えられる。

このpauseはupload consentの取消とは別である。temporary scheduling pauseとpolicy disableを同じstateにしない。

## Backend connection

`BackendWebSocketService`は次をnotification化する。

- version blocked
- update required
- connection state changed
- connection status
- disconnected
- connected

DB syncにはimmediate sync requestとdebounce用のlast-request timestampがある。WebSocket eventまたは明示requestで差分syncを促す構造が強く示唆される。

## Airgap

`AirgapModeStore`は`airgapModeEnabled`を保存し、settings modelは現在値とlaunch時値を分ける。

UIは次を明言する。

- restart後に有効
- telemetryを抑止
- update checkを抑止
- remote favicon requestを抑止

再起動が必要であるため、launch時のdependency assemblyでnetwork serviceを作らないか、起動snapshotを各serviceへ渡す設計が強く推定できる。

一方、次がairgapで確実に止まるかは未確認である。

- Backend WebSocket
- authentication
- DB sync
- video upload
- Sentry / TelemetryDeckの全path

`--no-video-upload`は別の起動flagであり、airgapと同一ではない。

## OpenBriefへの採用判断

cloud sync自体はMVPへ入れない。x870のLM Studio送信を含むoutbound policyだけを先に明示する。

```rust
enum OutboundPolicy {
    Disabled,
    LocalOnly {
        allowed_endpoints: Vec<EndpointId>,
    },
    InternetAllowed {
        allowed_providers: Vec<ProviderId>,
    },
}
```

LM Studio / Tailscale endpointは`LocalOnly` allowlistで許可する。private networkだから自動許可せず、request payloadとretentionをtask policyへ結び付ける。

将来のcrate候補:

```text
openbrief-outbound-policy
openbrief-sync
openbrief-evidence-upload
openbrief-upload-receipt
```

実装するまではcrateを作らず、ADR上のboundaryだけ維持する。

## 主なevidence

| Address | 観測 |
|---|---|
| `0x101231762` / `0x101231999` | sync pipeline pause / resume |
| `0x10124933f` | database sync status fields |
| `0x101247bae` | video upload status fields |
| `0x100d3aa1b` / `0x100d3aa30` | AX sync phase / status |
| `0x10123e095` | sync work limiter |
| `0x10020cc80` | production DB read-only sync pool |
| `0x100eb3b30` | debug data uploadを避けるsuspend guard |
| `0x1012705e3` | signed-out startup decision |
| `0x101232086` | upload consent change notification |
| `0x100318b68` | delete-after-upload setting保存 |
| `0x1006447d4` / `0x100644bf4` | overlay時pause / resume |
| `0x100ec1fb0`–`0x100ec20a0` | Backend WebSocket notifications |
| `0x100ec29b0` | airgap setting key |

## 未確認

- delete-after-uploadの正確なACK条件
- queue persistence、retry、backoff、idempotency key
- restart後のqueue resume
- airgapが停止する全subsystem
- signed-out時のcapture policyをedition横断で統一できるか
- server側retention、authorization、tenant isolation
