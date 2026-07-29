# Screenpipe source reference

## 固定判断

Screenpipe全体をforkせず、現行crateへも依存しない。必要になった時だけ、最終MIT commitに存在する小さく独立したmoduleを、license noticeとsource SHA付きで評価する。

特にOpenBriefの最優先platformであるniriについて、Screenpipeはforeground window sourceを提供しない。現行の`grim` / Portal captureもMIT終了後に追加されたため、OpenBriefはniri event sourceとcapture adapterを独立実装する。

## 調査基準とlicense

| 項目 | 値 |
|---|---|
| Repository | [screenpipe/screenpipe](https://github.com/screenpipe/screenpipe) |
| Source調査snapshot | [`d114e14bac7e52b5aa8aab46d130ca48e04aba6a`](https://github.com/screenpipe/screenpipe/tree/d114e14bac7e52b5aa8aab46d130ca48e04aba6a) |
| 最終MIT commit | [`892199f742e46d0c5d9e8c06687b35ca7c2b6547`](https://github.com/screenpipe/screenpipe/tree/892199f742e46d0c5d9e8c06687b35ca7c2b6547) |
| License切替 | [`81e412ff5315dd7f6e270bed1911fadb2de5dc44`](https://github.com/screenpipe/screenpipe/commit/81e412ff5315dd7f6e270bed1911fadb2de5dc44) |
| 調査日 | 2026-07-30 |

2026-06-10以後のrepository全体にはScreenpipe Commercial Licenseが適用される。personal / non-commercial / research利用等は許されるが、商用製品への組込み、配布、競合製品利用には別契約が必要である。

過去にMITで公開されたversionはMITのまま残る。ただし、codeを使う場合は`892199f…`に存在するblobだけを取得し、MIT notice、元path、commit SHAをthird-party noticeへ記録する。現行sourceを見て同じfile名の旧MIT blobへ遡るだけでは、後から追加された実装を利用できない。

## Repository map

現行workspaceは`crates/*`へ分割されている。OpenBriefに近い範囲だけを示す。

| 領域 | Source | 責務 |
|---|---|---|
| Screen / OCR | [`screenpipe-screen`](https://github.com/screenpipe/screenpipe/tree/d114e14bac7e52b5aa8aab46d130ca48e04aba6a/crates/screenpipe-screen) | monitor capture、OCR、frame comparison |
| Paired capture | [`paired_capture.rs`](https://github.com/screenpipe/screenpipe/blob/d114e14bac7e52b5aa8aab46d130ca48e04aba6a/crates/screenpipe-capture/src/paired_capture.rs) | screenshot、a11y、OCR、JPEG、DB commit |
| Accessibility | [`screenpipe-a11y`](https://github.com/screenpipe/screenpipe/tree/d114e14bac7e52b5aa8aab46d130ca48e04aba6a/crates/screenpipe-a11y) | input event、tree walk、private window判定 |
| Audio | [`screenpipe-audio`](https://github.com/screenpipe/screenpipe/tree/d114e14bac7e52b5aa8aab46d130ca48e04aba6a/crates/screenpipe-audio) | device、VAD、STT、diarization |
| Store / Search | [`screenpipe-db`](https://github.com/screenpipe/screenpipe/tree/d114e14bac7e52b5aa8aab46d130ca48e04aba6a/crates/screenpipe-db) | SQLite、FTS、migration、write queue |
| Orchestration | [`screenpipe-engine`](https://github.com/screenpipe/screenpipe/tree/d114e14bac7e52b5aa8aab46d130ca48e04aba6a/crates/screenpipe-engine) | recorder、API、CLI、retention、cloud |
| Event bus | [`screenpipe-events`](https://github.com/screenpipe/screenpipe/tree/d114e14bac7e52b5aa8aab46d130ca48e04aba6a/crates/screenpipe-events) | process内broadcast event |

crateが分かれていても、domainの結合が小さいとは限らない。`screenpipe-capture`はDB、snapshot writer、a11y、OCRへ同時に依存するため、OpenBriefのcapture backendとしてそのまま抜けない。

## Linuxとniri

現行Linux captureは概ね次へ分岐する。

```text
niri / sway / Hyprland等
  → grim

GNOME / KDE等
  → xdg-desktop-portal + PipeWire

fallback
  → xcap
```

分岐は[`monitor/linux_wayland.rs`](https://github.com/screenpipe/screenpipe/blob/d114e14bac7e52b5aa8aab46d130ca48e04aba6a/crates/screenpipe-screen/src/monitor/linux_wayland.rs)、Portal sessionは[`monitor/linux_portal.rs`](https://github.com/screenpipe/screenpipe/blob/d114e14bac7e52b5aa8aab46d130ca48e04aba6a/crates/screenpipe-screen/src/monitor/linux_portal.rs)にある。

しかし両方とも最終MIT版には存在しない。

- `linux_wayland.rs`分割: 2026-06-25
- `linux_portal.rs`: 2026-07-24
- 最終MIT版のLinux capture: 実質`xcap`

foreground sourceも再利用できない。

- [`focus_tracker/linux.rs`](https://github.com/screenpipe/screenpipe/blob/d114e14bac7e52b5aa8aab46d130ca48e04aba6a/crates/screenpipe-engine/src/focus_tracker/linux.rs)は`FocusEvent::Unknown`固定
- a11y Linux adapterはHyprland、Sway、X11を扱うがniri event streamを持たない

したがってOpenBriefは次を維持する。

```text
openbrief-source-niri
  niri msg --json event-stream
  → foreground event reducer

openbrief-capture-niri
  必要な範囲だけgrim等を実行
  → memory上の一枚
```

Screenpipeの現行`grim`実装をコピーするのではなく、timeout、stderr、temporary artifact cleanupをOpenBriefの要件から小さく実装する。

## 最終MIT版から評価できる小module

| Module | 内容 | OpenBrief判断 |
|---|---|---|
| [`window_pattern.rs`](https://github.com/screenpipe/screenpipe/blob/892199f742e46d0c5d9e8c06687b35ca7c2b6547/crates/screenpipe-core/src/window_pattern.rs) | include / exclude pattern | denylist semanticsを実装する時だけ比較 |
| [`activity_feed.rs`](https://github.com/screenpipe/screenpipe/blob/892199f742e46d0c5d9e8c06687b35ca7c2b6547/crates/screenpipe-a11y/src/activity_feed.rs) | contentなしactivity timestamp | input activityを追加する時に比較 |
| [`incognito/titles.rs`](https://github.com/screenpipe/screenpipe/blob/892199f742e46d0c5d9e8c06687b35ca7c2b6547/crates/screenpipe-a11y/src/incognito/titles.rs) | private window title例 | browser adapter導入時の補助資料 |
| [`text_normalizer.rs`](https://github.com/screenpipe/screenpipe/blob/892199f742e46d0c5d9e8c06687b35ca7c2b6547/crates/screenpipe-db/src/text_normalizer.rs) | FTS5 query sanitize / expansion | OCR全文検索を採用した時だけ |
| [`snapshot_writer.rs`](https://github.com/screenpipe/screenpipe/blob/892199f742e46d0c5d9e8c06687b35ca7c2b6547/crates/screenpipe-screen/src/snapshot_writer.rs) | JPEG、atomic rename、age cleanup | raw evidence保存を採用した時だけ |
| [`frame_comparison.rs`](https://github.com/screenpipe/screenpipe/blob/892199f742e46d0c5d9e8c06687b35ca7c2b6547/crates/screenpipe-screen/src/frame_comparison.rs) | histogram、SSIM、perceptual hash | 5分tick不足を実測した後だけ |

現MVPで直接code reuseする価値があり得るのは`window_pattern`程度である。それも小さいため、必要なsemanticsをtestで固定して独自実装する方がdependencyとlicense管理は単純になり得る。

## Patternだけ採るもの

### Triggerをcaptureから分ける

[`event_driven_capture.rs`](https://github.com/screenpipe/screenpipe/blob/d114e14bac7e52b5aa8aab46d130ca48e04aba6a/crates/screenpipe-engine/src/event_driven_capture.rs)はapp switch、window focus、click、typing pause、scroll stop、clipboard、visual change、idle、manualをtriggerとして正規化する。

OpenBriefはenumの考え方だけ採り、MVPは次へ限定する。

```text
ForegroundChanged
FiveMinuteTick
IdleChanged
LockChanged
PolicyChanged
```

click、key、clipboardはcontent収集範囲を増やすため入れない。

### A11y first、OCR fallback

現行`paired_capture`はa11y textが十分ならOCRをskipする。CPU削減には有用だが、screenshot、JPEG、DB commitまで一関数に結合する。

OpenBriefでは将来追加する場合も次を別traitにする。

```text
ContextSource
PixelCapture
TextExtractor
PolicyGate
EvidenceStore
```

### Bounded work

OCR semaphore、capture timeout、event debounce、write queue healthの考えは採る。ただしScreenpipeのglobal singleton event busや10,000件broadcastを移植せず、所有者が明確なbounded `mpsc` / `watch`を使う。

## 採用しない範囲

| 対象 | 理由 |
|---|---|
| repository全体のMIT巻き戻しfork | 現行との差分が大きく、niri改善はMIT後 |
| 現行crate dependency | licenseとmoving mainをcoreへ持ち込む |
| `paired_capture` | privacy、capture、OCR、disk、DBが結合 |
| `screenpipe-db` | video、audio、a11y、FTS向けでMVPには過剰 |
| `screenpipe-audio` | audioの製品判断自体が未完了 |
| engine CLI / localhost API | OpenBriefのsingle collector + small CLIより大きい |
| Pipes / cloud / team / sync | local-first MVPに不要 |
| 現行`OcrGate` / `text_regions` | MIT後の実装であり、MVPにも不要 |

## 再調査する条件

次のどれかが決まるまでは、この文書を参照してrepository全体を再調査しない。

1. OCR全文検索を実装する
2. raw screenshotをdiskへ保存する
3. audioを正式採用する
4. niri以外の二つ目のplatformを追加する
5. Portal / PipeWire backendが必要になる
6. event-driven captureが5分tickを有意に上回る
7. Screenpipeの商用ライセンスを取得する
