# 05. OpenBriefへの採用判断

## 結論

AttentionはScreenpipeよりOpenBriefのActivity Recallに近い先行例である。ただし、AttentionはmacOSでscreen historyを保存・検索する製品、OpenBriefはniriで疎なcaptureをlocal VLMへ送り、raw imageを残さず時間想起を助ける実験である。

したがって、classやschemaを移植せず、次の一般原則だけを独自実装する。

## 利用談で変わる評価

「今日何をしていたか」を返すだけなら、raw image非永続の現在MVPでよい。しかし、提供されたCoast Localの例は別の価値を示す。

| use case | summary-only MVP | raw evidence |
|---|---:|---:|
| 14時ごろ何をしていたか | 対応可能 | 不要 |
| 過去に見たpage/topicを思い出す | 多くは対応可能 | 場合による |
| 消えたform draftを復元する | 対応困難 | 必要 |
| presentationをslide/PDFへ再構成 | 不可能 | 必要 |
| Agentが以前のcommand/errorを精密確認 | 部分的 | keyframe/OCRが有利 |

したがって、Attentionのmedia storage資産は価値が低いのではなく、**OpenBriefの現在MVPとは異なるcapabilityを支えている**。

```text
Lane A: Activity Recall
  metadata + summary
  raw imageは即時破棄

Lane B: Local Agent Memory（MVP後、opt-in）
  短期keyframe ring + OCR/embedding + agent query

Lane C: Enterprise Insight
  別product/別consent。現在は対象外
```

Lane Bを試す場合も、最初から全録画videoにしない。24時間など短いretention、foreground keyframe限定、端末local、明示enable、Agentへread-only queryという条件で増分価値を測る。

## MVPから採用

### 1. Privacy判定をcapture前に行う

```text
tick
  ↓
lock / pause / denylist / focused targetを判定
  ├─ 不適格: gapだけ保存
  └─ 適格: screenshot取得
```

Attentionではscreen captureとAX observationの両方で除外を早期適用する。OpenBriefも`Excluded`を取得後filterではなくsource/capture境界のpolicyにする。

### 2. Bounded laneと明示的drop

Attentionはcapture slotとwrite queueをboundedにする。OpenBriefはさらに小さく、capture/VLM laneを1本だけ持つ。

queueへraw imageを積まない。busyなら`model_busy_local`、timeoutなら`model_timeout_local`をtimelineへ残す。

### 3. Commit時の世代確認

pause、delete、次captureの後に古いVLM responseが返ってもcommitしない。

```text
capture_id + policy_generation
    ↓ request
response
    ↓ 現在世代と一致?
yes: commit
no: discard
```

Attentionのin-flight予約とstale capture拒否から借りるが、Rustで独自実装する。

### 4. Contextの観測時刻を分ける

window event、screenshot、VLM responseは別時刻である。ActivitySliceには少なくとも`started_at`、`captured_at`、`summarized_at`を区別する。

## MVP後に採用候補

### 差分判定

captureを1分以下へ短縮する場合、Attentionの差分OCRに相当する`FrameComparer`を追加する。

- downscale hash
- changed pixel ratio
- 同一ならVLMをskip
- OCR/VLM結果を前frameから無条件copyしない

### Accessibility enrichment

AT-SPIをoptional backendとして追加する。

```text
niri event
  + screenshot/VLM
  + AT-SPI text snapshot
      ↓
ActivitySlice
```

最初はfocused applicationの浅いtextだけをmemory上で使う。全tree永続化、content-addressed graph、live observerは必要性が確認されてから追加する。

### Segment compaction

同じapp/title/summaryが連続するsliceをquery時またはcommit時に畳む。Attentionの`start_frame_id` modelは参考になるが、OpenBriefでは正確なwindow event時刻を保持する。

### FTS5

7日分のsummary、title、app IDを単純LIKEで探せなくなった時だけFTS5を追加する。OCR box、BM25 tuning、TF-IDF dedupは初期には不要である。

### Agent query CLI

人間用の`today` / `around`と同じread modelをAgentにもJSONで提供する。

```console
openbrief around 14:00 --json
openbrief search "LM Studio structured output" --json
```

Evidence Storeを導入した場合だけ、frame IDを明示指定する別commandを追加する。AgentへDB pathやretention delete権限を直接渡さない。

Attentionの追加解析では、Agent連携の実体がlocal CLI bridge、Agent用skill、外部Agentへのprompt routingに分かれていることが確認できた。OpenBrief MVPはこのうち**CLIと小さなread-only skillだけ**を採用する。常駐RPC、MCP、deep link、auto-sendは必要性が出るまで追加しない。詳細は[AI Agent連携](07-agent-integration.md)を参照する。

## 採用しない

| Attention | 理由 |
|---|---|
| screenshotのHEIC永続化 | MVPのraw image非永続要件と衝突。opt-in Evidence Storeでのみ再評価 |
| FFmpeg video compaction | keyframe ringの価値を確認する前は不要 |
| `window_bound`全window保存 | foreground想起には過剰 |
| OCR box全件保存 | VLM summary MVPでは不要 |
| Accessibility tree全保存 | privacyと容量が大きい |
| capture同時実行3 | x870が速くても順序とprivacyを優先し1本にする |
| macOS ScreenCaptureKit wrapper | niriではgrim/libwayshotを使う |
| AttentionのUI・timeline配置 | 独自UXを検証する |

## Crate境界への反映

Attentionのclass数をそのままmicro-crateへ変換しない。最初は次で足りる。

```text
openbrief-core
openbrief-source-niri
openbrief-capture-wayland
openbrief-model-openai
openbrief-store
openbrief-cli
```

crate内部module:

```text
capture-wayland/
  policy.rs
  grim.rs
  lane.rs

store/
  frame_order.rs
  activity.rs
  gap.rs
```

二つ目のOS/backendを追加する時にだけ、traitやcrateを増やす。

## 実装順

1. niri foreground eventをSQLiteへ順序付き保存
2. capture前denylist
3. lane 1のgrim captureとLM Studio request
4. capture IDによるlate response拒否
5. `today` / `around`表示
6. 実測後にframe diff
7. 必要ならAT-SPI enrichment

Attentionから最も早く借りる価値があるのは、画像処理algorithmではなく「早期privacy判定、bounded work、late result拒否」の三点である。
