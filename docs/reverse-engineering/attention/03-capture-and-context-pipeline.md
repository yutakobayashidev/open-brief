# 03. Captureとcontext取得

## Capture scheduling

`ScreenshotRecorder`のlogには、interval指定でrecordingを開始し、最大同時capture数を3にする記述がある。

```text
timer
  ↓
capture slot取得
  ├─ 空きあり: capture開始
  └─ 満杯: frameをdrop
```

**確認**:

- capture slot取得・解放をcountする
- queue full時はframeをdropし、累積drop数を記録する
- capture session終了後に返ったframeは破棄する
- pause deadlineを設定でき、期限後に再開する
- inactivity時は「frameへinactive flagを付ける」または「recordingをpauseする」pathがある

capture cycleは`exclusion`、content列挙、filter、snapshot、metadata、screenshot、window info、OCR、Accessibility、encode、storeへ分けて時間計測する。OpenBriefも`source_ms`、`capture_ms`、`model_ms`、`store_ms`だけは最初から分ける。

OpenBriefの5分tickでは同時実行数3は不要である。VLM laneを1にし、busyなら画像を作る前に`model_busy_local` gapを残す方が小さい。

## Capture前のprivacy判定

Attentionは除外を保存後のfilterだけにしていない。

**確認**:

- focused appが除外対象ならframeをskip
- focused windowが除外対象ならframeをskip
- ScreenCaptureKitへ渡すwindow集合から除外appを外す
- allowed windowが0件ならscreenshotを行わない
- focused browserの内容を安全に読めない場合にframeをskipするpathがある
- 除外appではAccessibility observationもskip
- Attention自身のwindowをcapture対象から除く

`ScreenCaptureService`はnative display sizeとtarget resolutionを分ける。逆コンパイルでは解像度計算へ1280を渡すpathがあり、縦横比を保って最大辺または幅を約1280pxへ抑える**強い推定**ができる。OpenBriefもVLM送信前に上限解像度を固定する。

**提案**: OpenBriefもdeny判定を`capture()`より前に行う。captureしてから画像を捨てる設計では、raw imageがprocess memoryやlog/error pathへ入る。

## Foreground attribution

**確認**:

- frameへbundle ID、bundle version、window title、URLを渡す
- window bounds、layer、z-orderを別tableへ保存
- windowlessなfrontmost appの場合、cursor下またはtopmost windowへframeを再帰属するpathがある

この再帰属はmacOS固有であり、niri向けOpenBriefへ同じheuristicを移植しない。niri event streamを時刻とforeground attributionのsource of truthにする。

## OCR pipeline

AttentionはApple Visionを使うpersistent OCR pipelineを持つ。

**確認**:

- `OCRExecutionGate`
- persistent OCR request
- full OCR
- previous frameとの差分OCR
- storage resolution変更時のcache clear
- changed regionのcropに失敗した場合はfull OCRへ戻る
- previous OCR segmentsと新規結果のmerge
- OCR preprocess / OCR / postprocessの個別計測

```text
current frame
  + previous frame
      ↓
changed regionを推定
  ├─ 成功: regionだけOCR
  └─ 失敗/size変更: full OCR
      ↓
previous OCR segmentsとmerge
```

これは高頻度captureで価値がある。OpenBriefの5分captureでは前frameとの差が大きい可能性が高く、MVPへそのまま採用しない。

`OCRExecutionGate`は一つの実行だけを許し、待機requestをFIFO continuationへ積む。AttentionではOCR backlogを処理するが、OpenBriefの周期captureでは待たせずbusy時にskipする。手動queryだけは待機可能にする。

## Accessibility pipeline

**確認**:

- frontmost appのAX treeを取得
- observerとlive treeを持つ
- pending change bufferがある
- node数または時間間隔によるpartial publish設定がある
- extraction resultへ`extraction_mode`と`is_partial_tree`を保存
- Electron、Gecko、WebKitなどでAccessibilityを起こすwakerがある
- user inputまたはinteraction recencyをtree buildのbudget判断へ使うsymbolがある
- AX callback用の専用thread/run loopがある
- app切替時の古い非同期登録をgeneration tokenで拒否する
- click/scroll由来のdirty signalを50msでcoalesceする
- scroll位置から意味のある祖先を最大10段辿る

**強い推定**:

```text
AX notification / input dirty
      ↓
PendingAXChangeBuffer
      ↓ coalesce
LiveAccessibilityTree
      ↓ budget内でbuild
partial tree ──→ frameへ先行付与
      ↓
complete tree
```

増分cacheがdriftした場合はfresh re-walkへ戻る。観測した判定は、probeが16件未満なら2件以上、16件以上なら約12.5%以上のdriftである。full rebuildには2秒cooldownがあり、user interaction中でも約5回の延期後は実行してstarvationを避ける。

静的解析だけでは、各AX notification、debounce値、capture timerとの正確な同期順は確定できない。

### SelectionCaptureService

名前に反して、画面矩形selectionではなく、frontmost applicationのAccessibility APIから現在選択中のtextを取得するserviceである。

**確認**:

- AX permissionを先に確認
- frontmost applicationとbundle IDを取得
- selected textをtrimし、空なら保存しない
- application metadataとcapture時刻を付ける
- 最後のselectionは30秒でexpire

選択textは「現在どこへ注意を向けているか」の強いsignalになり得る。OpenBriefではAT-SPI対応applicationだけに使うoptional `SelectedTextEnricher`としてMVP後に検討する。本文をlogへ出さず、VLM promptへ使った後に破棄する。

## ImageAnalysisManager

名前からVLM managerに見えるが、確認できた実体はVisionKitのLive Text / QR code解析である。

**確認**:

- textとmachine-readable codeを対象にする
- frame ID単位の`NSCache`
- 同じframeは再解析しない
- 新しい解析要求時に以前のdebounce taskをcancel
- timeline snapshot表示中は解析を延期
- feature無効やprewarm失敗をfatalにしない

OpenBriefは実装を移植しないが、「content hash単位のanalysis cache」「最新要求優先」「optional enricher失敗でcapture全体を止めない」というcontrol patternを借りられる。

## ScreenshotとAXの対応

`storeFrame(...)`はimage、capture timestamp、OCR segments、bundle ID、Accessibility tree、title、URL、window bounds、cursor、capture display rectを一度に受ける。

これはframeを観測の結合点にする設計である。ただし、capture、OCR、AXが完全に同一時刻のsnapshotであるとは限らない。`is_partial_tree`と`extraction_mode`を保持するのは、このずれを隠さないためと推定できる。

OpenBriefでもsourceごとに`observed_at`を保持し、VLM summaryとwindow eventを「同時だった」と偽装しない。
