# 09. Browser URLとprivate browsing

## 結論

Attentionはbrowser contextを一つの汎用AX queryで取っていない。

```text
focused browser window
  → AX permissionを毎回確認
  → browser bundle別adapter
  → bounded AXWebArea探索
  → URL / title / web content有無
  → private判定
       ├─ Automation: deterministic
       └─ local heuristic: non-deterministic
  → domain / private exclusion
  → capture eligibility
```

URLが読めない場合にwindow titleからURLを捏造する証拠はない。focused browserがunreadableならframe全体をskipし、background browserのfailureはwindow単位でconservativeに扱う。

## URL取得

入口`0x100366360`は毎回`AXIsProcessTrusted()`を確認する。起動時にpermissionがあっても、runtime revokeを別pathで再検査する。

AX applicationとfocused windowのmessaging timeoutは0.5秒である。各childを読むpathにも0.5秒timeoutがある。

### Browser dispatch

`0x100368130`はbundle IDからbrowser adapterを選ぶ。binaryに含まれる対象は次である。

- Safari
- Chrome、Canary、Beta、Dev
- Edge
- Arc
- Dia
- Brave
- Vivaldi
- Opera
- Firefox、Developer Edition、Nightly、Beta
- `org.mozilla.*`

adapterはAX children、group、scroll area、web area、split groupをbrowser familyに応じて辿る。generic fallbackも複数ある。

`0x100368f9c`はrecursiveなAXWebArea探索を行うが、`0x100369180`のpredicateでdepthとnode数をboundedにする。`AXURL`を持つWebAreaを見つけた場合と見つからない場合を別logへ残す。

Firefox系とDiaでは、PIDごとにmanual accessibility engineを一度activateするpathがある。

## Private browsing

### Directとheuristic

UI説明には、Automation accessがあればincognito stateを直接読み、なければ推定するとある。

private verdictは次を持つ。

```text
isPrivate
deterministic
recordedAt
```

AX window identityをkeyに最大32件cacheする。Automationが利用可能ならasync direct path、なければlocal heuristicへ進む。

heuristicには`incognito`、`private window`等のtitle literalを使う強い証拠があるが、全条件とcache TTLは確定していない。

OpenBriefではprivateをboolへ潰さない。

```rust
enum PrivateState {
    Private,
    Public,
    Unknown,
}

enum PrivateEvidence {
    AutomationDeterministic,
    TitleHeuristic,
    Unknown,
}
```

`Unknown`を`Public`としてcaptureしてはいけない。

## Browser window observation

`0x100364f64`はwindow単位に次をまとめる。

- URL
- AX title
- URLがない場合のweb content有無
- private verdict
- window identity

private windowのCGWindow IDを解決できないpathもlogする。titleは独立して取得するが、URL fallbackとしてtitleをparseする証拠はない。

## Health state machine

`BrowserURLReadHealthMonitor`はbundle IDとbrowser versionをkeyにする。

| 定数 | 値 | 意味 |
|---|---:|---|
| sustained failure | 5秒 | 一時的failureと継続failureを分ける |
| cooldown / re-signal | 60秒 | 同じfailure通知を連発しない |
| stale reset | 300秒 | 古いfailure系列をresetする |

telemetryはbundle ID、browser version、failure secondsを持つ。URLやtitle本文を送る証拠ではない。

## Capture policy

`ScreenshotRecorder`のbrowser handlingはforegroundとbackgroundを分ける。

- focused browser unreadable: frame全体をskip
- background browser unreadable: conservative handlingを行うが、全captureは止めない

domain exclusionはUserDefaultsからlistを読み、lowercase normalizationして照合する。private exclusionは別の`excludePrivateBrowsing`設定である。

domainとprivateの内部的な評価順は確定していない。ただし、両方ともwindow metadata解決後、capture候補の決定前に適用される。

## OpenBriefへの採用判断

MVPはniriのapp IDとwindow titleだけで開始し、browser URLを必須にしない。追加する場合は独立adapterにする。

```text
openbrief-source-browser
  BrowserAdapter
  BrowserObservation
  BrowserHealth
```

要件:

1. AX callにtimeout
2. recursive探索にdepth/node budget
3. browser bundle/version単位のhealth
4. permissionを継続再検査
5. focused unreadableはreason付きgap
6. private判定はstateとevidenceを分離
7. domain/private/app exclusion reasonを監査可能にする

AutomationはmacOS固有であり、Linux版ではbrowser extension、DevTools protocol、AT-SPI等を別adapterとして比較する。Attentionのbrowser別AX implementationを移植しない。

## 主なfunction

| Address | 意味 |
|---|---|
| `0x100366360` | AX trust確認とbrowser window入口 |
| `0x100368130` | browser adapter dispatch |
| `0x100368f9c` | bounded AXWebArea探索 |
| `0x100369180` | depth/node budget predicate |
| `0x10036a840` | manual accessibility engine activation |
| `0x10036aa64` | group系adapter |
| `0x10036ac9c` | group/scroll/web area探索 |
| `0x10036b760` | web area/split group探索 |
| `0x10036bbcc` | child AX read timeout path |
| `0x10036c25c` | nonempty AX string reader |
| `0x1003666b4` | private verdict cache |
| `0x100367394` | Automation direct path |
| `0x100367ca0` | local heuristic path |
| `0x100360f3c` | browser health monitor |
| `0x100361af8` | health decision |
| `0x10017a34c` | focused unreadable時のcapture skip |
| `0x10031a7c4` | domain exclusion照合 |

## 未確認

- private verdict cacheのTTL
- heuristicの全条件
- Automation scriptの内容
- domain/private exclusionの厳密な先後
- browser familyとadapterの全組み合わせ
