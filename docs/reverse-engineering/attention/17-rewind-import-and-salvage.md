# 17. Rewind import、migration、video salvage

## 結論

AttentionのRewind importerはsource DBを複製して終わらない。

- frame、video、OCR nodeという履歴factはsource IDと対応を保つ。
- application、domain、segmentというderived dataはAttentionの意味論で作り直す。
- sourceは変更せず、暗号化DBは一時plaintext DBへexportし、mediaはcopy-on-write cloneから読む。
- interruption後は中途半端なbatchを継続するよりpartial artifactをclean upし、安全な境界から再実行する。
- metadata importと、後日の欠損video salvageを分離する。

これはScreenpipe等からOpenBriefへimportする時のよい一般原則になる。

## Migration flow

```text
detect Rewind
  → validate schema
  → decrypt to temporary DB if needed
  → disk preflight
  → clone chunks
  → attach source
  → batch import facts
  → rebuild app/domain/segment
  → write completed markers
  → clean temporary artifacts
  → later: salvage missing videos
```

## Sourceを変更しない

schema validationは`FUN_1002d3698`、decryptは`FUN_1002dd6f8`、COW cloneは`FUN_1002ea8b8`である。

暗号化DBは元DBを更新せず、`sqlcipher_export('plaintext')`で`temp_rewind_decrypted.db`へexportする。migration SQLはsourceから読み、destinationだけへ書く。video chunksも元directoryから移動せずcloneを作る。

sourceを削除・更新する処理は確認できなかった。ただしOS levelのread-only open flagまでは未確認である。

## Schemaとkey

`PRAGMA user_version`と`PRAGMA application_id`を読み、必須tableとoptional tableを分けて存在確認する。SQL利用状況から必須は`frame`、`video`、`node`、optionalは`segment`、`doc_segment`、`searchRanking`と強く推定できる。

暗号化DBには外部keyが必要だが、keyの取得元は未確認である。version番号だけでhard rejectする比較も確定できなかった。

OpenBrief importerはversionだけでなく、必要table / columnのcapabilityで互換性を判定する。

## Fact mapping

### Video

- source video IDとpathを維持する。
- source frameの実数から`num_frames`を再計算する。
- frameがないvideoはimportしない。

主SQL: `0x100ebcb60`

### Frame

- source frame IDを維持する。
- `videoId`と`videoFrameIndex`をdestination参照へ写す。
- `text`をforeground、`otherText`をbackground OCRへ写す。
- titleとsearch rankingを写す。
- timestampをepoch millisecondsへ正規化する。

timestampはISO-like文字列なら`julianday`、数値ならUnix秒として`1000`倍する。主SQLは`0x100ebc790`。

### OCR node

node IDとframe IDを維持する。normalized座標をvideo width / heightでpixel座標へ確定し、text offset / lengthも保持する。主SQLは`0x100ebc420`。

## Derived dataの再構成

- application: lowercase bundle IDでdistinct化し、native IDを採番
- domain: URL hostをlowercaseで正規化してnative IDを採番
- segment: app、domain、URLのいずれかが変わるframeから再生成

主SQL:

- application: `0x100ebf510`
- domain: `0x100ebf7b0`
- segment: `0x100ebfca0`

source provenanceを保つfactと、製品固有のderived modelを分ける点が最重要である。

## Disk preflight

`FUN_1002df500`はfree bytes、cleanup可能量、DB migration見積り、video cloneのvolume、固定headroomを比較する。固定headroomは約3 GiBで、cross-volumeならCOW cloneを無料と見なさずvideo量を追加要求へ含める。

式の厳密な変数対応はSwift decompiler出力から完全には確定できなかった。OpenBriefではreflinkやhardlinkの成功を前提にせず、cross-volume copyのworst caseでpreviewする。

## Interruptionとrollback

- `rewind.migration.inProgress`を保存する。
- startupで中断を検出すると既知のpartial artifactを削除する。
- 今回作成したdestination DBだけを失敗時に削除する。
- 既存DBは保持する。
- native DBに既にframeがあれば重複importを避ける。
- duplicate migration callを拒否する。

cleanupは`FUN_1002da8a0`。永続checkpointから途中batchをresumeするpathは見つからず、安全な境界から再実行する設計と強く推定できる。

完了状態はDB metadataの`rewind_migration_completed`と、UserDefaultsのcompleted / timestamp / inProgressで二重化する。DBをauthoritative、設定markerをUX用cacheとするのが独自実装では明確である。

## Video salvage

`RewindVideoSalvageService`はimport済みvideoごとに次を行う。

1. destination fileがあればskip
2. source chunksにもなければskip
3. destination directoryを作る
4. sourceからcopy
5. 個別失敗を記録して次へ進む

file loopは`FUN_1002fae8c`、summaryは`FUN_1002fbc34`。`rewind.videoSalvage.attempted.v1` markerを持つ。

これは上書きsyncではなく、idempotentな欠損補修である。metadataを先に利用可能にし、巨大mediaを独立repair jobへ分けられる。

## OpenBrief importer contract

```text
detect
  → validate
  → preview
  → preflight
  → import into staging
  → verify
  → atomic finalize
```

source別crateは共通`Importer` traitを実装する。

```text
source_kind
source_record_id
import_run_id
captured_at_utc_ms
media reference
```

MVPではpartial resume checkpointを作らず、staging DBまたはstaging tablesへimportしてatomic finalizeする方が単純で安全である。

原則:

1. sourceは絶対に変更しない。
2. timestampを入口で一形式へ正規化する。
3. provenance IDとderived IDを分ける。
4. 既存destinationを失敗時に丸ごと削除しない。
5. media copyは非上書き・idempotentにする。
6. cross-volumeのworst-case容量を見積もる。
7. importerとmedia salvageを分離する。
8. finalize前にcount、timestamp range、参照整合性を検証する。

## 未確認

- decryption keyの取得元
- supportする`user_version`範囲
- image-only frameの別import path
- persistent partial-batch resume
- cross-volume clone failure時のfallback
- source DB openのOS-level read-only flag
- DB markerと設定markerが矛盾した時の優先順位
