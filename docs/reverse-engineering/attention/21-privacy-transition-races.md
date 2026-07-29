# 21. Privacy transition race

## 結論

Attentionはcapture前のprivacy判定を複数持ち、AX secure text、private browsing、notification banner、自前overlayへ個別対策を置く。一方、除外設定変更と既に進行中のcaptureをprivacy epochで結ぶcommit-time barrierは確認できなかった。

OpenBriefでは`Allow / Deny / Unknown`の三値policyと、capture予約時・commit時の二回検証を明示する。

## Pixel captureの二層guard

`ScreenshotRecorder`は次を保存前にskipする。

- focused windowがexcluded
- focused appがexcluded
- focused browser windowがunreadable
- 全windowがexcluded

background browserのwindow情報が読めない場合もconservativeに扱う。

主なlog:

- focused window exclusion: `0x100f347b0`
- all windows excluded: `0x100f34760`
- background browser unreadable: `0x100f34800`
- focused browser unreadable: `0x100f34850`
- focused app exclusion: `0x100f34900`
- allowed / excluded count: `0x100f34930`

focused metadataだけで判定せず、ScreenCaptureKitへ渡すwindow集合からもexcluded windowを外す。

## Runtime exclusion change

exclusion list変更時はfrontmost appを即再評価する。

- handler: `FUN_1000fb7e4`
- re-evaluation log: `0x100f33340`
- AX observer start / skip: `FUN_1000fa9f0`、`0x100f33250`

以後のAX observationは止まる。ただし変更前に開始済みのpixel capture、OCR、AX tree、write reservationをprivacy generationで一括破棄するpathは見つからなかった。

未確認race:

```text
Allowでcapture開始
  → exclusion設定を追加
  → old capture完了
  → write queueへcommit
```

実際の漏洩を確認したわけではない。commit-time privacy revalidationを静的に確認できないという結論である。

## Secure textとmasked password

AX builderはsecure text fieldのvalueを読まず、masked password文字列も除去して長さが漏れないようにする。

- secure field drop: `0x100f33440`
- masked value strip: `0x100f333e0`
- callers: `FUN_10011a0f4`、`FUN_100110870`
- role: `AXSecureTextField @ 0x100e9fb20`

これはAX dataへの対策であり、screen imageをpixel redactionする証拠ではない。画像側はapplicationが表示するmaskに依存する。

password manager appは専用groupとして既存exclusion preferenceへmigrationされる。

## Private browsing

private verdictは少なくとも次を持つ。

```text
isPrivate: Bool
deterministic: Bool
```

cache、TTL、最大cache size、AX scan depth / node上限がある。

- private scan: `FUN_100367ca0`、`FUN_1003674dc`
- direct / inferred verdict logs: `0x100f3a8e0`、`0x100f3a960`
- unresolved private CGWindowID: `0x100f3a9b0`

Automation permissionがあればbrowserから直接incognito stateを読み、なければAX scanで推定する構成である。

実質的なmodelはBoolではなく次になる。

```text
Allowed
Private
Unknown
```

private browsing exclusionが有効な時は`Unknown`もcapture不可にする。

## Notification banner

binaryにはNotification Center processのbundle IDと、messaging appを除外している時はnotification bannerも自動除外する説明がある。

- `com.apple.notificationcenterui`
- `com.apple.usernotificationcenter`
- Slack、Discord、Messages等のmessaging group

通知本文をOCRで判定するのではなく、Notification Center windowをcapture filterから外すprocess-level ruleと強く推定できる。

## Self overlay

自前overlay表示中はrecording gateを閉じ、hidden後に開く。overlayが消えたのにgateが閉じたままならself-healする。

- close gate: `0x100f3da20`
- open gate: `0x100f3da50`
- stranded gate recovery: `0x100f354f0`

## Logging

selection captureは選択文字列の先頭100文字とapplicationをloggerへ渡す。

- `FUN_1000a3328`
- log format: `0x100f32960`

formatでexplicit public指定はないためUnified Loggingではprivate扱いと考えられるが、secretをloggerへ渡す設計自体をOpenBriefでは採用しない。

privacy関連logの多くはbundle ID、件数、booleanで、window titleやURL本文を直接出す明確なlogは確認できなかった。

## OpenBrief privacy snapshot

```rust
struct PrivacySnapshot {
    epoch: u64,
    app: Decision,
    domain: Decision,
    private_window: Decision,
}

enum Decision {
    Allow,
    Deny,
    Unknown,
}
```

capture予約時にsnapshotを保持する。DB commit直前に次を再確認する。

```text
current epoch == captured epoch
app/domain/private decision == Allow
capture source PID/window == attributed PID/window
recording gate == open
```

一致しなければimage、OCR、AX、title、URL、VLM responseをまとめて破棄する。

## Fail-closed fixtures

1. capture開始後、完了前にappをexcludedへ変更。期待はDB、media、OCR、AXが0件。
2. focused browser URL取得がtimeout。期待はframe全体を破棄。
3. private verdictがnon-deterministic / timeout。期待は`Unknown`として破棄。
4. excluded messaging app上へnotification banner表示。期待はbanner pixelとOCRを保存しない。
5. `AXSecureTextField`へcanary入力。期待はAX value、masked length、logにcanaryなし。
6. focus変更とframe completionを競合。期待はsourceとmetadata PID不一致なら破棄。
7. overlay表示中とdismiss直後。期待はgate epochが一致するframeだけ保存。
8. selectionへcanary secret。期待は全logにsecretなし。

## 未確認

- exclusion変更時のin-flight capture invalidation
- notification auto-exclusion evaluatorの完全なcall chain
- image上のsecure field pixel redaction
- private verdict cache TTLの実値
- selected text logのruntime persistence
