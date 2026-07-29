# 16. Invocation、selection、overlay

## 結論

Attentionの低摩擦な呼び出しは、hotkeyを登録するだけではない。多重起動を防ぐUI state、Accessibilityからのclipboard非破壊な選択文字列取得、overlay chain全体のfocus restoration、自己UI表示中のcapture gateを一つのsessionとして扱う。

今回確認した`SelectionCaptureService`は矩形スクリーンショットではなく、focused elementの`AXSelectedText`を取得するserviceである。

## Hotkey flow

`FUN_10001a15c`はprimary / search hotkeyを解除・再登録し、設定に応じて次を切り替える。

- recorded shortcut
- double Command
- Shift + double Command
- Command連打系だけを無効化するpolicy

search handler `FUN_10001c41c`は、searchが既に開いている、開き途中、別UIが前面にある等を検査する。generation counterと`isOpeningSearch`で多重openを抑制し、状態遷移後にapplicationをactivateする。

関連entry:

- `handleOpenTimeline @ 0x1000147f0`
- `handleOpenSearchMode @ 0x1000145a8`
- `HotkeyModalGate @ FUN_1003b2308`

`HotkeyModalGate`はmodal panelやevent tracking中でもhotkeyを失わないようrun loopを進める。global hotkeyの低level実装がCarbon、CGEventTap、NSEvent monitorのどれを中心にするかは未確認である。

## Selection capture

main flowは`FUN_1000a3328`、AX helperは`FUN_1000a3bac`である。

```text
AX permission
  → frontmost application metadata
  → system-wide AX element
  → focused UI element
  → AXSelectedText
  → trim whitespace
  → selection + source app
```

focused elementには0.5秒のAX messaging timeoutを設定する。権限不足、focused elementなし、attributeなし、空文字列はすべてnilとして扱う。

clipboardを一時的に書き換えるfallbackは確認できなかった。OpenBriefでもselection providerが使えない時は明示的にunavailableを返し、clipboardを破壊しない。

## Overlay sessionとfocus restoration

- present: `FUN_100645ccc`
- close: `FUN_100644bf4`
- route / gate sync: `FUN_100645970`
- key event: `FUN_100646738`
- background click: `OverlayHostDimmingView.mouseDown: @ 0x1006425e0`

最初のoverlayを開く時だけ、次をfocus restoration stateへ保存する。

- applicationがactiveだったか
- 他の自前UIが見えていたか
- route stackが空だったか

animation中の追加operationはpending operationとして直列化する。overlay chainが空になった時点で、overlay前はAttentionがactiveでなく、現在はactiveで、他の自前UIが残っていない場合だけ自身をdeactivateする。特定PIDを直接activateするより、元applicationへ自然にfocusを返す構造と強く推定できる。

`OverlayWindow`はkey/main windowになれる通常の操作windowで、mouse位置や予定rectからmulti-display上の配置先を選ぶ。

## Self-capture exclusion

Attentionは二層で自己UIを除外する。

```text
通常capture
  → bundle / window exclusion

自前overlay表示中
  → recording gateを閉じる
  → overlay chain終了時にgateを開く
```

overlay present時はrecording gateを閉じ、uploadも一時停止する。closeまたはroute transition時にhost visibilityを再検査してgateを同期する。

通常capture側ではfocused bundle IDと除外集合を比較し、ScreenCaptureKitのwindow collectionもfilterする。

- overlay gate: `FUN_1006447d4`、`FUN_100644bf4`、`FUN_100645970`
- focused app exclusion: `FUN_100177a24`
- capture window filter: `FUN_100177fc4`、`FUN_10017a34c`

bundle exclusionだけではanimation中の半透明frameや別windowを確実に避けられない。自前UIのvisibility lifecycleとcapture gateを結ぶ方が安全である。

## OpenBriefへの採用

CLI段階ではglobal hotkey frameworkを作らない。niri、keyd等のbindingから次を呼べばよい。

```text
openbrief recall
openbrief search
openbrief capture-selection
```

commandはdaemonへ送る共通invoke eventに正規化する。selection取得はprovider traitへ分け、Linux a11yで取得できる時だけ使う。CLIはfocusを奪わずstdoutまたはTUIへ返す。

Tauri導入時の最小module:

```text
invoke-hotkey
selection-provider
overlay-session
capture-gate
```

`overlay-session`はRAII tokenにし、最後のtokenがdropされた時だけcapture gateを開く。保持するstateは次で足りる。

```text
was_app_active
had_visible_own_ui
route_depth
capture_gate_token
```

採用価値が高いのは、notch UIや特殊shortcutではなく次の三点である。

1. selectionをclipboard非破壊で取る。
2. overlayを独立window列ではなく一つのroute stackとして扱う。
3. 自己UI表示中はcapture gateを閉じる。

## 未確認

- global hotkeyの低level backend
- timeline handlerの完全な分岐
- screen capture exclusion modifierの具体的window API
- 矩形region screenshot capture
- 元application PIDを明示的にreactivateするfallback
- menu bar、floating window、overlay presenterの完全な責務境界
