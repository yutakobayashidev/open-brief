# 01. 解析対象と方法

## 対象

- 対象: ユーザーがGhidraへ読み込んだAttention macOS binary
- 調査日: 2026-07-29
- platform: macOS向けSwift application
- binary version / hash: Ghidra MCPから取得できず未記録

macOS向けであることは、AppKit、ApplicationServices、ScreenCaptureKit、Vision、ServiceManagementなどのframework pathとSwift symbolから確認した。CPU architectureと配布versionは、この調査だけでは確定していない。

## 使用した方法

- Ghidra MCPによるdefined strings、imports、functionsの確認
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

最も大きい不足は、対象binaryのversion、SHA-256、取得元が記録できていないことである。今後同じ調査を更新する場合は、解析開始前に次を保存する。

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
