# 12. Searchとretrieval pipeline

## 結論

Attention / Coastの検索は「OCRをFTS5へ投げ、score順に返す」だけではない。binary内のSQL、設定key、stage logから、次のpipelineが確認できる。

```text
query parse / validation
  → timeからframe ID範囲を解決
  → FTS match数をprobe
  → single passまたはstreaming chunks
  → BM25 raw hits
  → prefilter
  → title dedup
  → application/domain dedup
  → TF-IDF similarity dedup
  → metadata / OCR box enrichment
  → local OCR dedup
  → result conversion / streaming send
```

目的はrankingの精密化だけでなく、ほぼ同じ画面、new tab、Mission Control、同じtitle/domainの連続frameを人間とAgentへ大量に返さないことである。

## Query grammar

### 確認

query builderはFTS5のoperatorを事前validationする。

- 空のparenthesized groupを拒否
- operatorの前後にsearch termを要求
- `NOT`をbinary operatorとして扱う
- `OR NOT`を拒否し、parenthesesを使うよう案内
- unbalanced parenthesesを拒否
- unbalanced quotesを拒否

FTS SQLは次の形である。

```sql
SELECT rowid, bm25(ocr_fts) AS score
FROM ocr_fts
WHERE ocr_fts MATCH ?
  AND rowid >= ?
  AND rowid <= ?
```

app、domain、timeはFTS query stringへ混ぜず、segment/application/domain joinとframe timestamp条件でfilterする。domain suffix resolve用のqueryも別に存在する。

### Privacy上の観測

`escapeQuery`のlogは入力本文ではなく、入力長、escaped長、hashを記録する形式を持つ。一方、別のdebug logにはdeduplicated query stringを出すformatがある。production log levelは未確認だが、OpenBriefではsearch termそのものをlogへ残さない。

## Single passとstreaming

### 確認

検索前にmatch countをlimited subqueryでprobeする。

```text
matches <= singlePassHitThreshold
  → single pass
matches > threshold
  → frame範囲をchunk化してstreaming
filter-only query
  → probeをskipしてchunk streaming
```

stage logにはchunkごとのraw count、各dedup後count、latencyがある。古いchunkと新しいchunkのtimestamp順が逆転した場合をbugとして検出するlogも存在する。

streamingはUIのprogressive result表示だけでなく、長い履歴を一度にmemoryへ載せないbounded executionとして機能する。

## Prefilter

### 確認

設定には次がある。

- `prefilterNewTabs`
- `prefilterMissionControl`
- `prefilterExcludeApps`

runtime logはprocessed数、dropped new tab数、dropped Mission Control数を分ける。mission control判定失敗は検索全体をfatalにせず記録する。

### 強い推定

empty/new tabやMission ControlはOCR文字列がqueryへ偶然hitしても、本人が探すcontentそのものではないため落とす。excluded appはcapture前filterが主だが、migrationやlegacy data等を検索時にも守るdefense in depthと考えられる。

## Dedup stages

### Title dedup

設定:

- `titleDedupWindowHours`
- `titleDedupOneEveryN`

同じtitleが時間window内で繰り返す場合に代表だけを残し、完全に一件へ潰さず一定間隔で残す構造が示唆される。

### Application/domain dedup

設定:

- `appDomainDedupWindowSeconds`

同じapplication/domainの近接hitを短いwindowで畳む。titleが変わらないSPAや、同じdomainでの細かなframe差分を抑える役割を持つ。

### Similarity dedup

確認した設定:

- `enableSimilarityDedup`
- `similarityDedupWindowHours`
- `similarityThreshold`
- `similarityWindowSize`
- TF-IDF max features / min document frequency / max document frequency ratio

TF-IDFはsparse vectorとcosine similarityを使う。binaryにはcharacter Jaccard、bigram Jaccardも存在するが、どのdedup stageで常に使われるかは制御flow全体を確定していない。

### Local OCR dedup

OCR box、frame dimension、foreground/background textから局所的なOCR strip表現を作る。

設定:

- inclusion threshold
- score difference threshold
- x/y proximity
- context word上限
- context文字数min/max
- locality sigma
- window size

これは画面の一部だけが変わった連続frameで、同じOCR hitを何度も返さないための最終stageと解釈できる。

### Bounded comparison

similarityとlocal OCRのlogには次がある。

- intra-chunk capped count
- intra-chunk max scan
- cross-chunk capped count
- cross-chunk max scan

dedupを全件総当たりにせず、比較windowをboundedにしている。

## Degradation

### 確認

TF-IDF similarity providerが未初期化でもFTS検索は継続する。

```text
FTS available
  ├─ TF-IDF ready
  │    → full similarity dedup
  └─ TF-IDF unavailable
       → fast fallback dedup
       → similarityなしで結果を返す
       → initializationはbackgroundで継続
```

local OCR dedupやhighlight enrichmentを後回しにするpathもある。FTS tableが空、DB未接続、cacheなしをそれぞれ区別する。

これは検索のenhancementが壊れてもbaseline retrievalを止めない設計である。

## OpenBriefへの採用判断

### MVP

最初は次で十分である。

```text
time range
  + summary/title/appのFTSまたはLIKE
  → timestamp順
  → 同じcontextの隣接sliceだけcollapse
  → limit
```

Activity Recallのrecord数は疎であり、Attentionの多段dedupを最初から実装すると「必要な再訪まで消す」riskの方が高い。

### Evidence Store後

1分以下のkeyframe、OCR全文、長期retentionを導入した場合だけ、次の順に追加する。

1. new tab / excluded sourceのprefilter
2. 同一contextの短時間dedup
3. chunk streaming
4. titleのone-every-N sampling
5. similarity dedup
6. OCR位置を使うlocal dedup

各stageはdrop前後件数を計測し、golden queryでrecallを落としていないことを確認する。

### Agent query

Agentへはraw frame全件ではなく、bounded query結果と`has_more`を返す。Agentが自分で時間範囲を狭められるよう、resultにはtimestamp、app、title、summary、evidence IDを含める。

検索で重要なのは「賢いranking」を一度に作ることではなく、**baselineを止めず、重複を消しすぎたstageを観測して外せること**である。
