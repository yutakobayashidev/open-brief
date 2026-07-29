# 14. Evidence、代表frame、artifact recovery

## 結論

AttentionのEvidenceは、画像だけを保存する特別なdata modelではない。検索可能な`Frame`を参照し、静止画、動画内frame、purged placeholderの順で表示可能性を解決するlayerである。

代表frameは、時間範囲全体から一枚を選ぶのではなく、一定以上の長さを持つsegmentごとに一枚を選ぶ。これにより、presentation、terminal、browserなど複数の文脈を一つの長い区間から取り出せる。

## Segment sample

### 確認

`SegmentSample(startMs:endMs:minSegmentLength:bundleIds:domains:)`は次を返す。

```text
(frameCount, durationSeconds, selectedFrameId)[]
```

SQLは次の段階に分かれる。

1. 対象時間範囲のframe ID範囲を求める。
2. `segment`ごとにframe countとtimestamp範囲を集計する。
3. `HAVING COUNT(*) >= minSegmentLength`で短いsegmentを除く。
4. segmentを開始時刻順に並べる。
5. 各segment内を`ORDER BY id ASC LIMIT 1 OFFSET ?`で一枚選ぶ。

主なevidence:

- `SegmentSample` metadata: `0x1012378a0`
- range query: `0x100eb8330`
- segment aggregate: `0x100eb8660`
- group / minimum length: `0x100eb8740`
- representative frame query: `0x100eb8790`

`OFFSET`の厳密な式は未確認である。`frameCount`から中央付近を選ぶ可能性は高いが、静的解析上は強い推定に留める。

## Evidence data

`EvidenceV`は`Frame`を内包する。Frameには少なくとも次の検索・表示・復元metadataがある。

- frame ID、timestamp、segment ID
- application bundle ID、display name、title、URL
- static image path
- video path、video frame number、video status、video ID
- image width / height
- OCR text、foreground text length
- query match count / match ranges

したがって、Evidence Storeはbinary imageの置き場ではなく、**時系列上のframe参照と、そのframeを復元・説明するmetadata**として設計するのが自然である。

## Media resolution

`FrameExtractor`はstatic imageを先に読み、存在しない時だけvideo pathとframe indexから抽出する。

```text
Frame reference
    ├─ static image exists → load image
    ├─ local video exists  → extract indexed frame
    ├─ cloud-only / purged  → show placeholder
    └─ broken reference     → explicit failure
```

主なevidence:

- static image優先path: `FUN_1008ce840`
- video extraction failure: `FUN_1008cef5c`
- `PurgedFramePlaceholder`: `0x100d6d270`
- purged video skip log: `0x100f3ef50`
- preload path: `FUN_10038c340`、`FUN_10038e938`、`FUN_10038ec34`、`FUN_10038eddc`

Evidence一覧はframe ID群を先読みし、画像cacheを利用する。media unavailableを単一の`missing`へ潰さず、static、local video、cloud-only、brokenを区別する点が重要である。

## Active-window crop

frameごとのwindow boundsは別tableに保存される。

```text
window_bound(
  frame,
  application,
  window_title,
  x, y, width, height,
  window_layer,
  z_order
)
```

`FocusedWindowBounds(frameId:activeBundleId:activeTitle:screenArea:)`は候補windowとfallback levelを返す。ログからfull-screen window、first normal window、same-app overlayというfallback候補が確認できる。

cropに必要な値は次である。

- capture displayのglobal rect
- frame imageのpixel width / height
- windowのglobal rect
- active bundle ID / title
- 選択時のfallback level

window boundsがcapture rectと交差しない場合は明示エラーになる。全画面へ黙ってfallbackすると、Agentが意図しない他windowの内容まで扱うため、OpenBriefでも失敗理由を残す。

厳密な座標変換、Y軸反転、Retina scale、rounding方式は未確認である。独自実装ではsynthetic multi-display fixtureで検証する。

## Artifact recoveryに必要な最小record

```text
Evidence
  id
  frame_id
  captured_at_utc_ms
  segment_id
  source_kind: static | video | cloud_only | missing
  image_path?
  video_path?
  video_frame_index?
  video_status
  image_width
  image_height
  capture_display_rect?
  active_bundle_id?
  active_title?
  url?
  selected_window_bounds?
  crop_fallback_level?
  ocr_text?
  query_match_ranges?
```

最初の代表選択は単純でよい。

1. 時間窓をsegmentに分ける。
2. `min_segment_frames`未満を除く。
3. 各segmentの中央frameを一枚選ぶ。
4. static image、video frame、placeholderの順で復元する。
5. crop不能なら原因をmetadataへ残し、ユーザーが許可した時だけ全画面を提示する。

## OpenBriefへの境界

Activity Recall MVPのsummary-only storeを変更しない。Evidence Storeは明示opt-inの別laneにする。

```text
Activity Recall
  sparse capture → VLM summary → raw image破棄

Short-term Evidence
  selected frame → encrypted local retention → explicit expiry
                                      ↓
                              exporter / Agent tool
```

PDF生成や複数frameからの文書再構成はAttention binaryで確認できなかった。これはEvidence Storeではなく、Evidence列を消費する将来のexporterの責務にする。

## 未確認

- representative frameの`OFFSET`式
- cover sampleの選択式
- active-window fallbackの完全な優先順位
- crop座標の変換式
- cloud mediaの再取得とplaceholder解除条件
- PDF / slide再構成機能の製品内実装
