# 23. Production databaseの暗号化境界

## 結論

このbuildのAttention / Coast Localで、通常運用の`rem.db`はplain SQLiteである可能性が非常に高い。SQLCipherは同梱されているが、確認できた利用先はRewind import時の暗号化DB readerとplaintext stagingであり、production databaseのopen pathではない。

これはruntimeでDB headerを読んだ結果ではなく静的解析による強い推定である。ただし、productionの`DatabasePool`生成、reflection field、PRAGMA、Rewind専用SQLCipher pathを相互に照合しており、「SQLCipherがlinkされているからproduction DBも暗号化される」とは判断できない。

## Production open path

`FUN_10020a590`はactive database pathを通常のGRDB `DatabasePool` initializerである`FUN_100479d3c`へ渡す。この経路に次は確認できない。

- encryption key引数
- `PRAGMA key`
- cipher migration
- key生成またはKeychain lookup

`FUN_10020cc80`はproduction Application Support directoryへ`rem.db`をappendし、同じ通常`DatabasePool`でread-onlyの`productionPool`を開く。pathは概ね次になる。

```text
~/Library/Application Support/inc.attention.rem/rem.db
```

production `DatabaseService`のreflection metadata、`0x100ef2920`周辺には次のfieldがある。

- `dbPool`
- `productionPool`
- `isReadOnly`
- `activeApplicationSupportDirectory`
- icon、video、frame directory

`encryptionKey` fieldはない。

## PRAGMAとsidecar

production database設定として次を確認した。

| Address | SQL |
|---|---|
| `0x100eb3ad0` | `PRAGMA journal_mode = WAL` |
| `0x100eb3af0` | `PRAGMA synchronous = NORMAL` |
| `0x100eb3b10` | `PRAGMA busy_timeout = 30000` |

plain SQLiteという推定が正しければ、`rem.db-wal`と`rem.db-shm`もAttention自身の暗号化境界には入らない。OSのFileVaultはdevice全体のat-rest protectionであり、application-level DB encryptionとは別である。

file mode、DB header、WAL / SHMの実内容、SQLite temporary fileの配置はmacOS runtimeで未確認である。

## SQLCipherの実際の利用範囲

次はRewind migrationのmethod群へ局在する。

- `RewindDatabaseReader`
- `SQLCipherConnection`
- `encryptionKey` reflection field
- `databasePath13encryptionKey...` constructor
- `db-enc.sqlite3`
- `PRAGMA ...cipher_memory_security`
- `sqlcipher_export('plaintext')`
- `temp_rewind_decrypted.db`

`sqlite3_rekey` symbolはSQLCipher frameworkのexportとして存在するが、application callsiteは確認できない。

したがって、このbuildでSQLCipher frameworkを同梱する主目的はRewind import対応と考えるのが最も整合する。

## Keychain、logout、airgap

binary内の`keychainService`、`sharedKeychainService`、access group関連symbolはAppIdentityと認証設定の並びにあり、production DB keyとの接続は確認できない。

次もproduction DB pathには見つからなかった。

- `SecItem*` / `kSec*`を使うDB key lifecycle
- key rotation
- logout時のDB key破棄
- crypto-erasure
- `secure_delete`
- deletion後の`VACUUM`

logoutはcapture / syncのgate、airgapはlaunch-time network policyとして観測される。どちらもDBの暗号化状態を変更する証拠はない。

## Temporary dataとbackup

明示的なplaintext temporary DBはRewind migrationに存在する。`temp_rewind_decrypted.db`には成功時と失敗時のcleanup pathがある。

production DBについては次を確認できない。

- encrypted backup
- encrypted temporary store
- WALを含むbackup policy
- Time Machine等の外部backup除外
- crash時に残ったSQLite temporary artifactの回収

静的解析から「production dataが必ず漏れる」とは言えないが、application独自の暗号化やcrypto-erasureを安全性の根拠にはできない。

## OpenBriefへの判断

MVPではraw screenshotをdiskへ保存せず、保存対象をapp ID、時刻、短いVLM summaryへ絞る。このため、SQLCipher導入を最初の価値検証の前提にはしない。ただし「local-firstだから暗号化済み」と表現せず、次を明示する。

```text
MVP:
  store directory / fileをuser-only permissionにする
  raw image、request body、response bodyを永続化しない
  full-disk encryptionを運用上の前提として表示する
  DB / WAL / SHMがplainであることをthreat modelへ記録する

Evidence Storeまたはmulti-user配布前:
  DB、WAL、SHM、backupを一つのapplication encryption境界にする
  keyはOS secret storeへ置く
  startup時にplain SQLite headerを拒否する
  privacy resetと通常logoutを別commandにする
  plaintext stagingとorphan cleanupを故障注入testする
```

JSONLからSQLiteへ移る時に暗号化を後付けするのではなく、raw evidenceを初めて永続化するreleaseを暗号化導入のhard gateにする。

## Runtimeで残る確認

1. `rem.db`先頭16 byteがSQLite headerか
2. Application Support、DB、WAL、SHM、media、socketのownerとmode
3. WAL / SHMにOCR、title、URL、AX valueが平文で現れるか
4. logout、delete、airgap切替後のfile lifecycle
5. crash中断したRewind plaintext stagingのorphan cleanup
6. Time Machine、Spotlight、crash report等の外部複製
