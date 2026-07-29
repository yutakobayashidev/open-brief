# 19. Retention、delete、除外の完全性

## 結論

Attentionの`purge`は、履歴全体のprivacy deleteではない。video rowをpurgedへ変更してmedia fileを削除し、disk usageを減らす処理である。frame、segment、OCR / FTS、AX snapshotは残り、UIでは`PurgedFramePlaceholder`として時系列参照を継続する。

```text
video candidate
  → DB: status = purged, size_bytes = 0
  → media fileをbest-effort削除
  → frame / OCR / AX / segmentは保持
  → placeholderでtimelineを維持
```

したがって、OpenBriefでは`expire_media`と`forget_event`を別の操作として定義する。

## Retention model

`RetentionPreview`は少なくとも次を持つ。

- proposal
- affected video count
- estimated bytes freed
- oldest affected timestamp
- re-encodeだけではstorage capへ到達できないか

`RetentionResult`はarchived videos、purged videos、affected frames、errorを持つ。proposalにはtime-basedとstorage-cap系がある。

主なmetadata:

- `RetentionPreview @ 0x101241069`
- `RetentionResult @ 0x101239ef7`

## Time-based candidate

candidateはvideo rowの作成時刻ではなく、そのvideoに属する最後のframe timestampで判定する。

```text
video
  + MIN(frame.timestamp)
  + MAX(frame.timestamp)
      ↓
newest frame < retention cutoff
```

新しいframeを含むvideoを古いIDだけで削除しない。

- aggregate SQL: `0x100ec4cf0`、`0x100ec4e70`
- purge loop: `FUN_1003d43b0`

## Purge ordering

per-video purgeはDB transactionを先に完了し、その後fileを削除する。

- transaction entry: `FUN_1003d9cc4`
- transaction closure: `FUN_1003da67c`
- DB mutation: `FUN_1003f566c`
- file deletion: `FUN_1003d9dfc`

DB側はvideo pathとstatusを読み、local mediaならpurged statusと`size_bytes = 0`へ更新する。fileが既にない場合は成功扱いになる。

file削除失敗は外へ再throwされず、DB rowはpurgedのまま残る。つまりDB state上のpurge完了と、physical media deletion完了は一致しない可能性がある。

## 残るdata

Retention pathから次の削除は確認できなかった。

- `frame`
- FTS row
- OCR boxes / text
- `ax_snapshot` / AX nodes
- segment / application / domain

`framesAffected`は削除frame数ではなく、purged / archived videoにより表示可能性が変わるframe数である。

## Archive recovery

archiveは中間statusを持つ二相処理である。

```text
active
  → archiving
  → temporary archive生成
  → DBをarchived mediaへ更新
  → original削除
```

- per-video archive: `FUN_1003d55a4`
- source欠損時のpurged化: `FUN_1003d56b8`
- orphan archive finalize: `FUN_1003d9458`
- archiveなしでactiveへrollback: `FUN_1003d97cc`

起動時にarchive fileが存在すればfinalizeし、なければactiveへ戻す。これはDBとfileを照合するrecoverable state machineである。

## Orphan cleanup

orphan cleanupは破壊処理をfail closedにする。

- temporary incomplete videoは削除
- DBが参照しないvideoはTrashへquarantine
- DB path取得失敗時は何も削除しない
- DB pathが0件なのにnon-temporary fileが存在する場合も削除しない

主処理は`FUN_1003e9aa4`。DB failureと本当に参照がない状態を区別する点は採用価値が高い。ただしTrashへの移動はsecure deletionではない。

## Storage cap

storage capはNULLの`size_bytes`をfilesystemからbackfillする。それでもNULLなら推測せず対象からskipする。

Delete modeは超過量を計算して古いvideoからpurgeする。Re-encode modeは解像度scaleから削減量を見積もり、全candidateを処理しても不足する場合は`capUnreachableWithReencode`を返す。

容量を満たせない時に黙って追加削除せず、policy変更を要求できる構造である。

## Integrity circuit breaker

retention整合性違反時はfailure build numberを保存し、time-basedとstorage-capの両方を停止する。

- setting: `settings.storage.retentionIntegrityFailedBuildNumber`
- time-based block: `FUN_1003c9c74`
- storage-cap block: `FUN_1003caec4`
- trip / clear: `FUN_100319338`

単なる再起動では解除せず、より新しいbuildでだけclearする。壊れたdelete logicを同じbuildで繰り返さないためのrelease-level circuit breakerである。

`retention_purge_size_mismatch` eventはあるが、mismatch計算からtripまでの完全なcall chainは未確認である。

## WALとphysical deletion

確認:

- main DBはWAL mode
- generic checkpoint support
- `PRAGMA wal_checkpoint(TRUNCATE)`文字列

未確認:

- retention直後のcheckpoint
- `VACUUM`
- `PRAGMA secure_delete`
- `auto_vacuum`

Retentionはそもそもframe / OCR rowを消さないため、privacy deleteではない。将来rowをDELETEするpathがあっても、SQLite pageとWALからのphysical eraseは証明できない。

## Retroactive exclusion

除外変更時に確認できるのはfuture capture policyのreloadとfrontmost appの再評価である。

- recording exclusion reload: `0x100f35250`
- domain exclusion reload: `0x100f35110`
- frontmost app re-evaluation: `0x100f33340`

既存historyをbundle / domain条件で検索して削除するsurfaceは見つからなかった。retroactive exclusionは未実装という強い推定になる。

## OpenBriefのdelete contract

```text
expire_media(evidence_id)
  → raw image / videoだけを期限切れ
  → metadataとsummaryは維持

forget_event(event_id)
  → frameをrootに全derived dataを削除
  → media deletion完了までdurable retry
  → old generationのlate resultを拒否
```

原則:

1. media expiryとprivacy deleteを別API・別文言にする。
2. `forget_event`はsummary、embedding、OCR、AX、segment、artifactをmanifestで列挙する。
3. `deleting → deleted`というrecoverable stateを持つ。
4. file deletion failureを成功表示せずdurable retryへ残す。
5. orphan cleanupはDB矛盾時にfail closedにする。
6. 除外追加時に「今後のみ」「過去も削除」を明示する。
7. delete開始時にprivacy epochを進め、古いcaptureをcommitさせない。
8. privacy delete完了時にWAL checkpointを行う。
9. 短期Evidenceはartifactごとのkey破棄によるcrypto-erasureを検討する。

## 未確認

- purge size mismatchの厳密な式とcircuit breaker call
- retention直後のWAL checkpoint
- 別のuser-facing full-history delete path
- physical secure deletion
- media file削除失敗の恒久retry
