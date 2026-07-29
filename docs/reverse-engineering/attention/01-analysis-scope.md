# 01. 解析対象と方法

## 対象

- 対象: ユーザーがGhidraへ読み込んだAttention macOS binary
- 調査日: 2026-07-29〜2026-07-30
- platform: macOS向けSwift application
- product: Coast Local Lite
- version: `1.0`、build `131000`、client resource `client-v00.00.131-lite`
- architecture: arm64 Mach-O
- main binary SHA-256: `adbf673733e411fc8b51625f0d5f2ede7b3f9e7ec64618b162377c45d3b03a45`
- bundled CLI SHA-256: `5b2ddc8a943b7022abbb30c10970a9241e79f121b1a198105a30c3d3d4b02cf7`
- bundle identifier: `inc.attention.rem`
- signing team identifier: `6U2JW3D8N3`

macOS向けであることは、AppKit、ApplicationServices、ScreenCaptureKit、Vision、ServiceManagementなどのframework pathとSwift symbolから確認した。architectureと配布versionはlocal app bundleから確認した。

metadataはGhidra projectと同じlocal distributionに含まれるapp bundleの`Info.plist`、`ClientVersion.txt`、Mach-O header、embedded signature、SHA-256から取得した。Ghidra MCPがload中programのhashを返さないため、Ghidra上のprogramとこのbinaryがbyte-for-byte同一であることはMCP経由では再検証できていない。ただしproduct identifier、symbol、address空間、bundled resourceは一致する。

## 使用した方法

- Ghidra MCPによるdefined strings、imports、functionsの確認
- Ghidra 11.3.2 headlessによるbundled CLIの別program importと逆コンパイル
- Swift reflection metadataとclass名の確認
- SQL migration、CREATE TABLE、index、triggerの確認
- runtime log formatからfailure pathとqueue policyを確認
- 一部functionの逆コンパイル

次は実施していない。

- applicationの起動、UI操作、通信観測
- debugger、dynamic instrumentation、permission bypass
- account、backend、private APIへの接続
- credential、token、user dataの取得
- binary、逆アセンブル、逆コンパイル結果のrepository保存

## 主要framework

**確認**:

- `ScreenCaptureKit`: screenshot/window capture
- `ApplicationServices`: macOS Accessibility API
- `Vision`: OCR
- `AVFoundation`, `CoreMedia`, `CoreVideo`: media処理
- `ImageIO`, `CoreImage`, `CoreGraphics`: image処理
- `CryptoKit`: hashまたは暗号関連。用途は未確定
- `ServiceManagement`: login item / background lifecycleの可能性
- `SwiftUI`, `AppKit`: UI

binary内にはGRDBの型、migration table、SQLが含まれ、SQLite accessにGRDBを使用していることも確認した。

## 解析上見えたmodule

| module | 観測した責務 |
|---|---|
| `AttentionApplications` | app/domain除外 |
| `AttentionUtils` | capture、OCR、Accessibility、storage、retention |
| `AttentionShared` | 共通modelとscreen-capture exclusion view |
| `AttentionSettings` | capture、retention、privacy設定 |
| `AttentionTimeline` | frame、segment、OCR/AX表示 |
| `AttentionPermissions` | Screen Recording、Accessibility permission |
| `AttentionMenuBar` | pauseなどのmenu bar操作 |
| `CoastShell` | application shellとCLI/agent連携の一部 |

`Coast`という旧称または内部名称が残っていると推定するが、製品履歴はbinaryだけでは確定しない。

## 再現性の不足

対象bundleのversion、build、architecture、hash、team identifierは追加調査で記録できた。bundled CLIは記録したhashのfileをGhidra 11.3.2へ新規importした。main binaryについては、既存Ghidra MCP programが同じhashであることの機械的照合と、その既存projectを作成したGhidra versionの記録が残っている。今後同じ調査を更新する場合は、解析開始前に次を保存する。

```text
product version
build number
Mach-O architecture
SHA-256
code-signing team identifier
Ghidra version
analysis date
```

repositoryにはこれらのmetadataだけを保存し、第三者binaryやdecompiler出力は置かない。
