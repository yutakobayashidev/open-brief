# 04. Storage、検索、retention

## SQLite schema

AttentionはGRDB migration付きSQLiteを使う。

主要table:

| table | 主なfield |
|---|---|
| `application` | bundle ID、version、display name、icon、dominant color |
| `domain` | normalized domain、common name、icon |
| `segment` | start frame、application、domain、URL |
| `frame` | timestamp、video/indexまたはimage path、size、OCR text、title、segment、inactive、capture display |
| `ocr` | frame、bounding box、text offset/length |
| `window_bound` | frame、application、title、geometry、layer、z-order |
| `video` | path、width、height、frame count、status |
| `ax_snapshot` | frame、root hash、timestamp、app、PID、mode、partial flag |
| `ax_node` | node hash、payload |
| `ax_node_edge` | parent hash、child hash |

## Segment model

`segment`はapplication、domain、URLが変化したframeだけを開始点として保存する。

```text
frame 100  app=A URL=/issue/1  → segment 10
frame 101  app=A URL=/issue/1  → segment 10
frame 102  app=B               → segment 11
```

SQL migrationには`LAG()`で前frameのapplication/domain/URLと比較し、いずれかが変わった時だけsegmentを作る処理がある。同一metadataが連続する重複segmentをmergeするmigrationもある。

OpenBriefでは、niriのfocus eventがすでに境界を与えるため、frameからsegmentを再構成しない。ただし「連続する同一contextを一つのsliceへ畳む」考えは採用する。

## OCR検索

**確認**:

- SQLite FTS5
- `foreground`、`background`、`title`をcontent tableと同期
- insert/update/delete trigger
- `porter unicode61 remove_diacritics 2`
- BM25検索
- OCR boxによるhighlight
- local OCR dedup
- FTS vocabularyを使うTF-IDF系dedup準備

検索pipelineにはtitle、app/domain、similarity、local OCRの複数dedup段階があり、各段階の件数と時間を計測するlogがある。

OpenBrief MVPはsummaryとapp/titleへの単純検索で十分である。OCR全文検索が必要になった時だけFTS5を追加する。

## Accessibility tree storage

旧schemaはframeごとのtree blobを持つ。新schemaは次である。

```text
ax_snapshot(frame_id, root_hash, metadata)
      │
      ▼
ax_node(hash, payload)
      │
      ▼
ax_node_edge(parent_hash, child_hash)
```

同じbutton、document、toolbarなどが連続frameで変化しなければ、node payloadをhashで共有できるcontent-addressed graphと解釈できる。

OpenBriefでAT-SPIを導入する場合も、最初から全treeを永続化しない。まずVLM summaryの補助へ使い、tree保持が必要だと実測できた場合だけ同様の差分化を検討する。

## Imageからvideoへの二段階storage

**確認**:

1. capture frameをHEICへencode
2. `image_path`付きframeとして保存
3. background compactionが画像をFFmpeg stdinへ送る
4. video rowと`video_index`へ置き換える
5. `image_path`をNULLにする
6. timeline表示時は静止画がなければvideoからframeを抽出する

FFmpeg processにはstartup verification、連続失敗cooldown、timeout、stderr capture、orphan recoveryがある。

この設計は大量frameを長期保存する製品には合理的だが、raw screenshotを保存しないOpenBriefとは目的が違う。

## 順序付きcommit

captureは並列に完了し得るため、Attentionはtimestamp順を守る層を持つ。

**確認**:

- capture開始時にin-flight reservation
- 完了時にwrite queueへ追加
- より古いin-flight captureが残る間は後続commitを待つ
- reservationなしで遅く到着したcaptureをreject
- stale reservationをwatchdogがdrop
- write queue overflow時はframeをdrop

OpenBriefでVLM requestを1本に限定しても、timeout後のlate responseは発生する。generation IDまたはcapture IDをcommit時に検証する設計は採用価値が高い。

timeline側もasync range query開始時のgenerationを保持し、結果が返った時に世代が変わっていれば古いwindowを破棄する。query resultはboundedなframe範囲と`hasMoreBefore/After`、retention/archived cutoffを持つ。

## Retention

Attentionはtime-based retentionとstorage capを持ち、actionは少なくともarchive/re-encodeとpurgeを含む。

破壊的処理の整合性異常を検知すると`retention integrity circuit breaker`をtripped状態にし、新しいbuildでclearされるまでstorage management全体を止める。

OpenBriefではraw mediaを保存しないため同じ機構は不要だが、delete countや対象範囲が期待と違う時に削除を停止する原則は採用する。

## Disk space guard

`DiskSpaceMonitor`は周期監視だけでなく、frame writeのdisk-full errorから即時checkへ入る。

**確認**:

- monitorの二重起動を避ける
- low-spaceではuserへ継続/停止を提示するpathがある
- criticalではrecordingを自動pauseする
- recovery時は空き容量を再確認し、条件未達ならpauseを維持する
- disk-full frameはtransaction rollbackし、対応imageもcleanupする

OpenBriefでもSQLite、log、optional embeddingがdiskを圧迫し得る。mediaを持たなくても、`StorageGuard`によるENOSPC即時pauseと明示gapは採用価値がある。
