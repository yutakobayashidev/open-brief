# 02. 全体アーキテクチャ

## 確認できたdata flow

```text
ScreenCaptureKit
  ├─ screen/window filter
  └─ screenshot
        │
        ├─ focused app / title / URL / window bounds
        ├─ cursor / mouse metadata
        ├─ Vision OCR
        └─ Accessibility tree
              │
              ▼
        ordered frame commit
              │
       ┌──────┴──────────┐
       ▼                 ▼
 SQLite / GRDB        HEIC image
 frame/segment/       temporary recent frame
 OCR/AX/FTS               │
                         ▼
                  FFmpeg compaction
                         │
                         ▼
                       video
```

このflowは、class名、`storeFrame(...)` signature、SQL schema、runtime logを組み合わせた**強い推定**である。

## Local memoryとcloud insight

提供された利用談とbinary内のCoast CLI説明を組み合わせると、少なくとも次の二つのproduct surfaceがある。

```text
Coast Local
  local recording + OCR/FTS + CLI
      ├─ 人間が過去を探す
      └─ coding Agentが過去作業をqueryする

Attention cloud
  organization-level work context / insight
```

この二つを同じprivacy判断で扱ってはいけない。個人端末内で本人のAgentが検索することと、組織が従業員のactivityを集約することでは、同意、目的制限、access control、retentionのriskが大きく異なる。

今回のbinary解析はlocal capture/storageを中心にしており、cloud backendのdata flow、authorization、enterprise analyticsは確認していない。

## Capture layer

**確認**したclass:

- `ScreenCaptureService`
- `SelectionCaptureService`
- `ScreenshotRecorder`
- `ScreenshotService`
- `CaptureThrottler`
- `ImageAnalysisManager`
- `OCRExecutionGate`
- `OCRManager`

`ScreenCaptureService`には少なくともscreen captureと、bundle identifier、title、frameを指定するwindow capture相当のsymbolがある。`SCShareableContent`、`SCContentFilter`、`SCScreenshotManager`、`SCStreamConfiguration`も参照される。

## Context layer

**確認**したclass:

- `AccessibilityObserver`
- `AccessibilityInputDirtyMonitor`
- `AccessibilityTreeBuilder`
- `AccessibilityTreeExtractor`
- `AccessibilityTreeService`
- `LiveAccessibilityTree`
- `PendingAXChangeBuffer`
- `AXObserverThread`

Accessibilityはscreen readerと同じmacOS AX APIを、画面上の意味構造を取得するsourceとして使用する。色やpixelではなく、role、title、value、selection、tree関係を取得できる一方、機密テキストも取得し得る。

## Policy layer

**確認**したclass:

- `ExcludedAppsService`
- `ExcludedDomainsService`
- `ExclusionGroupsService`

user指定のbundle ID/domainに加えて、groupと自動除外ruleが存在する。messaging appを除外した場合にnotification bannerも自動除外する説明文字列がある。

## Persistence layer

中心entityは次である。

```text
application ─┐
domain ──────┼─ segment ─ frame ─ OCR boxes
             │              ├──── window bounds
             │              ├──── HEIC or video position
             │              └──── AX snapshot
             │
             └─ timeline grouping/search
```

`segment`はframe範囲そのものを重複保存せず、`start_frame_id`とapplication/domain/URLを持つ。次segmentの開始までが継続時間になる構造である。

## Reliability layer

**確認**:

- capture同時実行上限
- capture queue full時のdrop
- in-flight capture予約
- timestamp順序を壊す遅延captureの拒否
- bounded write queue
- stale reservation watchdog
- disk full時のtransaction rollbackとimage cleanup
- retention integrity circuit breaker

Attentionは「すべてのframeを必ず残す」より、「順序とstorage整合性を壊さず、過負荷を明示的な欠損にする」方を選んでいる。

これはOpenBriefのfail-closed gap設計と近い。

## Agent-facing surface

binaryには、recording内のOCR、application、domain、session、frameを検索できるCoast CLIに加え、次の三層が確認できる。

1. SwiftNIOとnewline decoderを使うlocal CLI bridge
2. Claude、Codex、Cursor、OpenClawへCoast CLIの使い方を配るAgent skill
3. Coast内の`/agent` requestをdesktop app deep linkまたはterminal CLIへ渡すrouting

提供された利用談では、Claude CodeとDevin CLIが明示指示なしでもこのCLIをmemoryとしてqueryしたとされる。ただし今回のbinaryにDevin専用integrationはなく、自律的にqueryを選ぶAgent側の判断も確定できない。詳細は[AI Agent連携](07-agent-integration.md)で分離して扱う。
