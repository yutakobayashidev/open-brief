# 20. Capture trigger state machine

## 結論

Attentionのtimeline captureはevent-drivenではない。開始直後に一枚captureし、その後は既定2秒のrepeating timerだけでcapture taskを起動する。app、window、input、AX notificationはcaptureを前倒しせず、次の周期captureでmetadataとして観測される。

```text
start
  → immediate capture
  → every 2 seconds
      ├─ slot available → capture
      └─ 3 active       → drop
```

負荷時にintervalやresolutionを自動で下げる証拠はなく、backlogを増やさずhard dropする。

## Periodic trigger

- initializer: `FUN_100173e94`
- default interval: `2.0`秒
- max concurrent: `3`
- start: `FUN_100174c1c`
- immediate / timer capture entry: `FUN_100175658`
- timer callback: `FUN_100175c58`
- throttler acquire: `FUN_10016e7b8`

startはgenerationを確認してstateをresetし、一度captureしてから同じintervalのNSTimerを作る。各tickはSwift Taskを作り、active slotが3なら画像取得前にdropする。

`ScreenshotRecorder`へapp/window/input/AX eventから入る追加capture entryは見つからなかった。window切替の瞬間を正確に保存するのではなく、最大約2秒後のsampleへ反映する方式である。

## Idleとresume

idle monitor:

- poll: `5.0`秒
- default idle threshold: `4.0`秒
- monitor start: `FUN_1001902f8`
- detector: `FUN_100190780`

`pauseOnInactivity=true`ならinactive時にrecorderを止め、activity復帰後のreconcileでstartし直す。startは即時captureを行うため、復帰がcapture triggerになる。

falseならtimerを止めず、frameへ`is_inactive`を付ける。

## Late resultの三層防御

### Start generation

非同期start準備中にstop / resetされた場合、start generation不一致で開始をcancelする。

### Capture session UUID

stop `FUN_100176b4c`は次を行う。

- recording flagをfalse
- start generationをclear
- capture session IDを新UUIDへrotate
- capture taskをcancel
- timerをinvalidate

古いsessionのframe completionは正常なcancelとして破棄される。

### Timestamp reservation

storageはcapture timestampをin-flight reservationへ登録する。先行captureが残る間は後続commitを待ち、reservationなし、または既commit frameより古い結果をrejectする。watchdogがstale reservationを除去する。

- reservation: `FUN_10029e320`
- commit validation: `FUN_1002a19d0`
- watchdog: `FUN_10029e824`

generation、session、storage orderingを別層にする点が重要である。

## Multi-monitor selection

各tickでScreenCaptureKitのdisplay一覧を取り直すため、hotplugは次のtickで自然に反映される。

display選択順:

1. frontmost appのlayer 0で有効boundsを持つwindow中心
2. そのpointを含むCGDisplayとSCDisplayを照合
3. `NSScreen.main`
4. SCDisplayの先頭

主処理は`FUN_100177120`。一回のtickで一つのdisplayだけをcaptureする。全display同時録画ではない。

windowless appはcursor下またはtopmost windowへmetadataを再attributeする別pathを持つ。

## Resolutionとquality

default:

- QHD `2560 × 1440`
- balanced quality

preset:

| Preset | Maximum |
|---|---:|
| UHD | 3840 × 2160 |
| QHD | 2560 × 1440 |
| FHD | 1920 × 1080 |
| HD | 1280 × 720 |

quality値はhigh 80、balanced 65、space saver 50。custom dimensionsは320〜7680へclampする。

onboarding用の一時resolution overrideはあるが、CPU、latency、queue pressureによるadaptive interval / resolutionは確認できなかった。

## Manual captureは別経路

- manual screen capture: `FUN_1000981f0`
- manual window capture: `FUN_10009afa4`
- selection: AX text取得

manual captureは`ScreenCaptureService`のon-demand pathで、timeline recorderのtimer、task set、throttlerと合流する証拠がない。pause、exclusion、private browsing policyを完全に共有するかは未確認である。

これはOpenBriefで避けるべき分岐である。manual captureもperiodic captureも同じprivacy gateへ入れる。

## OpenBrief trigger design

triggerを最初から型にする。

```text
Periodic
WindowChanged
ApplicationChanged
DisplayChanged
ActivityResumed
Manual
```

MVP:

```text
periodic tick
  + niri app/window event
      ↓ 1–2秒coalesce
privacy preflight
      ↓
one capture in flight
  + latest one pending
      ↓
VLM
      ↓
commit-time privacy epoch validation
```

Attentionの3並列captureより、VLM用途では`1 in flight + latest pending`が小さく、古い画面を処理し続けない。

原則:

1. requestへgeneration、session ID、captured time、triggerを付ける。
2. window eventを短いdebounceでcoalesceする。
3. queue pressureではdropし、勝手にqualityを変更しない。
4. display selectionをadapterへ分け、fallback reasonを保存する。
5. stop / privacy changeでgenerationをrotateする。
6. manual captureも同じpolicy gateを通す。
7. selection取得はscreen captureを暗黙に起動しない。

## 未確認

- intervalを変更するuser setting
- onboarding resolution overrideのduration
- ScreenCaptureKit error retry / backoff
- manual captureのprivacy gate共有範囲
