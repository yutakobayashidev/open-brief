# 18. Telemetry、airgap、update、onboarding

## 結論

Attentionはlocal capture capabilityと、analytics、updater、remote enrichmentを別のnetwork capabilityとして扱う。Airgapは再起動後にnetwork側をまとめて止めるが、local recordingとsearchを止める説明ではない。

OpenBriefが借りるべき核心はtelemetry event名やguided tour UIではなく、次の分離である。

```text
capture capability
  ≠ analytics capability
  ≠ updater capability
  ≠ remote enrichment capability
```

## Telemetry

app固有analyticsはTelemetryDeckを利用する。確認したevent:

- `app_launch`
- `daily_heartbeat`
- `airgap_toggled`
- `permission_prompt_shown`
- `permission_granted`
- `onboarding_step_completed`

`AnalyticsHeartbeatService`の`FUN_10034e18c`で確認できたpayload:

- install age
- guided tour完了
- screen recording permissionの有無
- excluded app / website / categoryの件数
- CLI invocation count
- search count、zero-result count、result open count
- timeline external open count
- shortcut use count

screen image、OCR / AX本文、window title、URL、search query本文をheartbeatへ含める証拠は見つからなかった。ただし、全経路でcontent telemetryがないことを静的解析だけで証明したわけではない。

TelemetryDeck SDK endpointとlocal cache文字列はbinaryにあるが、SDK同梱物だけからruntime送信先を断定しない。

## Sentry

Sentry SDK、crash handler、ANR、replay関連classはlinkされている。一方、app固有DSN、`SentrySDK.start`の起動経路、screenshot添付、session replay有効化、capture contentを渡す処理は確認できなかった。

したがって確認事項は「SDKが同梱される」までであり、「本番で有効」「画面を送る」は未確認である。

## Airgap

主なkey:

- `airgapModeEnabled @ 0x100ec29b0`
- `_airgapEnabled @ 0x100f024f1`
- `_airgapAtLaunch @ 0x100f02500`

UI説明は、telemetry、update check、remote favicon requestを抑止し、再起動後に有効になるとする。launch時snapshotをnetwork subsystemの構成へ反映する設計と強く推定できる。

各requestが変更中の設定を読むより、process lifetimeでnetwork policyを固定する方がraceと部分適用を避けやすい。`airgap_toggled` eventがON操作直後に送られるか、OFF時だけかは未確認である。

OpenBriefでは起動時に次を一つのimmutable policyへ解決する。

```text
NetworkPolicy
  analytics: denied
  updates: denied
  remote_enrichment: denied
  model_endpoint:
    local_machine | tailscale_host | denied
```

x870のLM Studioはlocal-firstでもoff-device egressである。airgapとmodel endpoint policyを同じbooleanへ潰さず、送信先を明示する。

## Updateとedition

Sparkle frameworkとLite edition向けupdate serviceがある。

- scheduled check policy: `FUN_100863880`
- feed URL accessor: `FUN_10086312c`
- onboarding中のscheduled checkを拒否

手動checkまで拒否する証拠はない。AirgapのUI仕様ではupdate checkを止めるが、Sparkle service直前のruntime guardは未特定である。

settings metadataにはedition集合があり、account、connections、airgap等のUI capabilityをcomposition時に切り替える形跡がある。remote feature flag endpointは確認できなかった。

CLI MVPではGUI updater、remote flag、edition分岐を追加しない。

## Onboarding

guided tourはframe数が4を超える、つまり5 frame以上になった時にreadyとなる。空のtimelineを先に説明せず、実dataを少し作ってから価値を見せる。

Screen Recording、Accessibility、Input Monitoring、Finder Automationは別capabilityとして扱われ、permission prompt / grantedとclick countを個別に記録する。runtime revoke時にはpermission wizardへ戻る。

OpenBrief CLIではguided tourを作らず、次のprogressive flowで足りる。

```text
openbrief doctor
  → sourceとpermissionを説明

openbrief capture start
  → 最初の観測を作る

5 records captured
  → recall exampleを一度だけ案内

openbrief recall today
```

## OpenBriefへの採用判断

### 最初から採用

- analyticsをcapture pipelineと別crate / featureにする。
- telemetry型にはevent名、count、booleanだけを許す。
- screenshot、OCR、title、URL、query本文をtelemetry型で表現できなくする。
- network policyを起動時に確定する。
- permissionをcapability別に診断する。
- 実timelineができてからrecallを案内する。

### 後回し

- Sentry
- GUI updater
- remote feature flags
- edition別機能
- elaborate guided tour
- remote favicon

## 未確認

- TelemetryDeckへ送る全eventと全field
- SDK endpointのruntime override
- Sentry production initialization
- Airgap toggle eventの送信timing
- AirgapからSparkleへ至るruntime guard
- remote feature flagの有無
