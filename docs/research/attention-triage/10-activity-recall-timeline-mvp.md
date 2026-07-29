# 10. 入力不要のActivity Recall Timeline MVP

## Status

- 改訂日: 2026-07-29
- primary user: terminal / Vim / AI coding agentを使い、browser、docs、複数terminalを頻繁に行き来するFounder
- client: Linux、Wayland、niri 26.04
- inference host: x870上のLM Studio、Tailscale越しのOpenAI互換API
- UI: CLI only。Tauri、IDE extension、常時dashboardは対象外
- 判断: **goalやsession開始を要求せず、foreground windowの時刻と5分ごとの疎な画面観測から、今日いつ何をしていたかを15分単位で返す**

## 一文で言うMVP

初回に一度だけ有効化する。

```console
openbrief enable
```

以後はgoal、要件、agent command、windowごとの許可を入力しない。普段どおり複数windowを行き来し、必要になった時だけ確認する。

```console
openbrief today
openbrief around 14:00
```

OpenBriefはniriのforeground transitionから時刻と滞在時間を決定し、5分ごとにforeground window一枚をx870のLM Studioへ送って短い`ActivityObservation`へ変換する。raw画像は保存しない。

## 何を解決するか

primary jobは、作業開始時に意図を宣言することでも、終了時に日記を書くことでもない。

> 手入力なしで「今日の14時ごろ、何をしていたか」を30秒以内に思い出せること

開発中に次が起きることを正常な状態として扱う。

- 要件がまだなく、調査しながら問いを作る
- Ghostty、Firefox、Obsidian、別terminalを同じ作業内で往復する
- 短い寄り道と必要な資料確認を、window切替だけでは区別できない
- 時間経過の感覚が弱く、後から時刻と作業内容を再構成しにくい

goal、manual mark、agent hookは利用できれば補助情報になるが、本MVPの開始条件にはしない。

## 研究から置く制約

[08 awesome-adhd synthesis](08-awesome-adhd-cross-report-synthesis.md)では、時刻、終了、再開手掛かりを外部化し、思い出す作業を減らすことを共通原則としている。一方で、保存、検索、整理そのものが新しい実行機能負荷になり得る。本MVPは入力を要求せず、既定表示を今日の有限な15分timelineへ絞る。

[09 Window Transition MVP](09-window-transition-mvp-reset.md)で確認したとおり、一つの開発作業はterminal、browser、docs、複数terminalへまたがる。window switchは正確な時刻signalにはなるが、集中、脱線、task境界のproxyにはしない。内容は疎な画面観測で補い、観測できない区間をmodelに埋めさせない。

screen captureがADHD一般へ有効だという直接証拠はない。Founder本人の時間想起を改善するかというN-of-1として、metadataだけのR0と画像を加えたR1を比較する。治療、診断、症状検出は主張しない。

## 以前の案から変えること

| 以前 | 新MVP |
|---|---|
| `run --goal ... -- codex`でsession開始 | user serviceを一度enableし、日常作業を受動観測 |
| 開始windowだけ選択し、他は`allow-current` | foregroundの全windowを通常対象とし、app denylistで除外 |
| child command終了が観測の終端 | 日付、lock、idle、pauseを自然な境界にする |
| `Done / Open / Next`がprimary output | 時刻付きActivity Timelineがprimary output |
| confirmed NextからResume | ReflectionとResumeはtimeline検証後の別projection |

Codex wrapper、required goal、window個別許可、Reflection confirm、Return AnchorはMVPから外す。

## ユーザー体験

### 一度だけ有効化

```console
$ openbrief enable
Activity recall enabled.
Capture: foreground window every 5 minutes
Excluded apps: 1password, signal, discord
Raw screenshots: never stored
Model: x870 / configured-vlm-id
```

`enable`はuser configとLM Studio接続を検証し、systemd user serviceを有効化してcollectorを開始する。LM Studioが利用できなくてもwindow metadataの収集は開始できる。

app denylistはuser configへ一度だけ置く。日々のwindow切替ごとに許可を求めない。

### 今日を見る

```console
$ openbrief today
10:00–10:15  Ghostty 9m / Firefox 6m
  OpenBriefのLM Studio adapterを編集
  Structured Outputの仕様を確認

10:15–10:30  Ghostty 13m / Obsidian 2m
  capture policyとfixtureを編集
  不明: 変更がcommit済みか

10:30–10:45  private / excluded 8m, idle 7m
  内容は記録していません
```

時刻と分数はniri eventから算出し、modelには生成させない。内容が観測できなければ推測で埋めず、`不明`または`capture gap`と表示する。

### 指定時刻の前後を見る

```console
$ openbrief around 14:00
13:45–14:15
  13:47 Ghostty
  13:54 Firefox
  14:03 Obsidian
  14:08 Ghostty

観測
  LM Studioのimage inputを調査
  Rust HTTP clientのtest failureを確認
```

`today`は15分bucketの短い一覧、`around`は指定時刻前後30分の詳細である。既定でraw screenshotや全window titleを表示しない。

### 一時停止

```console
openbrief pause --for 30m
openbrief resume
```

password、DM、個人情報を扱う前にすぐ止められる。pauseはidempotentで、`--for`を付けた場合だけ自動再開する。

## 観測pipeline

```text
niri foreground event
        │
        ├─ exact timestamp / duration
        │        └─ local FocusSegment
        │
5-minute wall-clock tick
        │
        ├─ paused / locked / idle / excluded → captureしない
        └─ eligible foreground window一枚
                  │
                  └─ x870 LM Studio
                           │
                           └─ ActivityObservation
                                      │
                                      └─ raw frame即時破棄

FocusSegment + ActivityObservation
        └─ deterministic 15-minute ActivitySlice
                  ├─ openbrief today
                  └─ openbrief around <time>
```

periodic tick、将来のmanual capture、Agent requestはcapture backendを直接呼ばない。全てを`CaptureIntent`へ正規化し、同じ`PolicyGate`でreservation時とresponse / commit直前に検証する。capabilityは現在画面を読む`live_screen_read`と、保存済みevidenceを読む`history_read`に分ける。MVPは`live_screen_read`のperiodic originだけを公開し、他originも同じdeny結果になるcontract testだけを持つ。

### Window metadata

niri eventを受けたclientが次を決定する。

- foreground開始・終了時刻
- app ID
- process lifetime中だけ有効なopaque window key
- `observed | excluded | idle | locked | paused`の区分

window title、URL、terminal textはmetadata eventへ保存しない。switch回数、app滞在時間から集中、脱線、生産性、task境界を推定しない。

### Sparse capture

MVPのcapture ruleは一つにする。

> 5分tick時点のforeground windowを一枚だけcaptureする

次の場合はcaptureを作らない。

- collectorがpausedまたはdisabled
- screen lock中
- keyboard / pointer idleが5分以上
- foreground appがdenylist対象
- foregroundが切り替わってから10秒未満
- 前のcapture / VLM requestがまだ実行中

複数monitor全体、background window、video、audioは取得しない。window切替ごとのcaptureや画像の類似判定は、5分tickで短い活動を落とすことが実測された場合だけ後から検討する。

capture / VLM laneは一つに固定する。前のrequestが実行中なら新しい画像を作らず`model_busy_local` gapを残し、raw imageをmemory queueまたはdisk queueへ積まない。

### ActivityObservation

LM Studioには一枚の画像とapp IDだけを送り、次の形で返させる。

```json
{
  "schema_version": 1,
  "activity": "OpenBriefのLM Studio adapterを編集している",
  "visible": [
    "terminalにRust sourceとtest failureが表示されている"
  ],
  "inferred": [
    "adapterのerror handlingを修正中の可能性がある"
  ],
  "unknowns": [
    "変更が保存またはcommit済みかは分からない"
  ]
}
```

modelには時刻、duration、window key、心理状態を出力させない。OpenBriefがcapture ID、captured at、app IDを追加し、JSON Schema、文字数、control characterを再検証する。

### ActivitySlice

日付をlocal timezoneの固定15分bucketへ分ける。各bucketへ次を機械的に入れる。

- FocusSegmentから算出したapp別滞在時間
- bucket内のActivityObservationから最大3件のactivity
- `unknowns`から最大2件
- excluded、idle、locked、model failureのgap

隣接bucketの意味的cluster、task名の推定、embedding、vector searchはMVPでは行わない。同じactivity文字列のexact duplicateだけ除く。

## CLI surface

```text
openbrief enable
openbrief disable
openbrief status [--json]
openbrief today [--date YYYY-MM-DD] [--json]
openbrief around <HH:MM> [--date YYYY-MM-DD] [--minutes 30] [--json]
openbrief pause [--for <duration>]
openbrief resume
openbrief delete (--today | --date YYYY-MM-DD) [--force] [--no-input]
openbrief watch
```

### Semantics

- `enable`: configを検証し、systemd user serviceをenable / startする。既にenabledなら成功する。
- `disable`: serviceをstop / disableする。保存済みdataは削除しない。
- `status`: `enabled / paused / model unavailable`、最終window event、最終Observation時刻だけを表示する。
- `today`: local storeだけを読み、現在日または指定日の15分bucketを表示する。network requestを行わない。
- `around`: 指定時刻の前後を表示する。`--minutes`は5〜120、既定30。
- `pause`: captureとcontentを持つmetadata取得を止め、genericなpaused区間だけ残す。
- `resume`: pauseを解除する。既にactiveなら成功する。
- `delete`: 対象日のevent、Observation、ActivitySliceを連鎖削除する。
- `watch`: collectorをforegroundで実行する。systemd unitも同じcommandを使う。

`-h / --help`、`--no-color`、`-v / --verbose`は全command、`--version`はrootで受ける。primary dataはstdout、diagnosticとprogressはstderrへ出す。`today`、`around`、`status`だけstableな`--json`を持ち、successとerrorの両方に`schema_version`を付ける。machine timestampはoffset付きRFC 3339に固定し、query range、response byte数、result countへ上限を置く。

`delete`はTTYで確認し、non-interactiveでは`--force --no-input`を両方要求する。Ctrl-C時は進行中frameを破棄し、短いcleanup後に終了する。

systemd上の`watch`をcollectorとstoreの唯一のwriterにする。`pause / resume / status / delete`は`${XDG_RUNTIME_DIR}/openbrief/control.sock`のmode `0600` Unix socketでcollectorへ要求し、local HTTP / WebSocket serverは作らない。

### Config

user configだけを使い、projectごとのgoal fileは作らない。

```toml
[capture]
excluded_apps = ["1password", "signal", "discord"]

[model]
model = "configured-vlm-id"
origin = "http://<x870-tailscale-ip>:1234"
credential_ref = "lm-studio-x870"
```

場所は`${XDG_CONFIG_HOME:-$HOME/.config}/openbrief/config.toml`とする。credential本文はOS secret storeへ置き、config、CLI flag、environment、logへ入れない。

## Privacyとretention

| Data | 既定保持 |
|---|---:|
| raw screenshot | request memory中だけ |
| excluded appの内容、app ID、title | 保存しない |
| FocusSegment | 7日 |
| ActivityObservation | 7日 |
| ActivitySlice | 7日 |
| content非保持の実験metric | 3日間のexperiment終了まで |

- raw frameは成功、失敗、timeout、pause、shutdownで即時破棄する。
- retry queue、body log、thumbnail cache、crash recovery imageを作らない。
- `delete`はderived dataを含めて対象日を削除する。
- exclusion変更、lock、pause、delete開始時に`privacy_epoch`を進める。capture requestは開始時epochを保持し、commit直前に一致しなければimage、model response、title、summaryをまとめて破棄する。
- periodic、manual、Agent captureは同じ`PolicyGate`を通す。manual captureを「保存しないから安全」と例外扱いしない。
- `live_screen_read`と`history_read`を別capabilityにし、retroactive purge後はknown frame IDでも取得不能にする。
- media期限切れとprivacy deleteを同じ`purge`操作へまとめない。
- lock、idle、excluded、pausedの区間は内容なしのgapとして表示する。
- browserを許可するとbrowser内のDMやsecretを区別できない。MVPはFounderの非機密作業で試し、即時pauseとapp denylistを安全境界にする。
- sensitive content自動検出へ安全性を依存させない。

OpenBriefはLM Studio内部のstorageを管理できない。synthetic secretでLM Studioのhistory、log、temporary dataをMVP開始前とversion更新後に監査し、forensic zero-retentionは主張しない。

MVPのlocal storeはuser-only permissionにし、full-disk encryptionを運用前提として表示する。ただし、これをapplication-level encryptionとは呼ばない。raw evidenceをdiskへ保存する機能を追加する場合は、DB、WAL、SHM、backupを同じ暗号化境界へ入れ、keyをOS secret storeへ分離することをrelease gateにする。

## x870のLM Studio

OpenBriefはx870のLM StudioへTailscale IPv4で直接接続する。独自backend、reverse proxy、Tailscale Serve、server crateは置かない。

```console
lms server start --bind <x870のTailscale IPv4> --port 1234
```

使う標準endpointは二つだけである。

| Method | Path | 用途 |
|---|---|---|
| `GET` | `/v1/models` | enable時の接続とmodel確認 |
| `POST` | `/v1/chat/completions` | 画像からActivityObservationを生成 |

- LM Studioの`Require Authentication`を有効にし、Bearer tokenを使う。
- `image_url` data URLと`response_format.type = json_schema`を使う。
- requestは90秒でtimeoutし、自動retry、別model、cloud fallbackを行わない。
- LM Studio障害時もFocusSegmentと`today`は動作し、該当bucketを`content unavailable`にする。
- response body、image、prompt、window contentをlogへ出さない。

参考: [LM Studio OpenAI互換API](https://lmstudio.ai/docs/developer/openai-compat)、[Structured Output](https://lmstudio.ai/docs/developer/openai-compat/structured-output)、[API token認証](https://lmstudio.ai/docs/developer/core/authentication)

## Rust crate境界

最初は9 crate、1 binary、1 systemd user serviceとする。

```text
openbrief-core            FocusSegment、ActivityObservation、ActivitySlice、policy
openbrief-source-niri     niri event、lock、idle
openbrief-capture-api     Frame、CaptureBackend trait
openbrief-capture-niri    foreground window capture
openbrief-model-api       ActivityObservation contract
openbrief-model-openai    LM Studio Chat Completions adapter
openbrief-store           JSONL、retention、cascade delete
openbrief-app             watch / today / around orchestration
openbrief-cli             clap、human / JSON output、systemd操作
```

```text
cli → app
      ├─ core
      ├─ source-niri
      ├─ capture-api ← capture-niri
      ├─ model-api   ← model-openai
      └─ store
```

micro-crateはprocess境界ではない。一つのCLI binaryを配布し、OpenBriefの常駐processは同じbinaryの`watch`だけにする。LM Studio以外のHTTP serviceを増やさない。TauriはGo後に`openbrief-app`を呼ぶadapterとして追加する。

常駐service、bounded lane、atomic stateの参考実装とaudioの採用判断は[11 qwen-audio-agent調査](11-qwen-audio-agent-assessment.md)に置く。source codeはcopyせず、Linux systemd lifecycleとsingle process ownerのpatternだけを採用する。

ScreenpipeとEntire CLIのsource-level採否は[OSS implementation references](../../implementation-references/README.md)へ固定した。Screenpipeは全体forkせず、niri source / capture adapterを独立実装する。Entireはhook event正規化、pure lifecycle、CLI UXだけを参考にし、daemonless process modelとGit checkpoint storeは採用しない。

## Golden Case

- [GC-02 Activity Recall happy path](../../../fixtures/golden-cases/gc-02-activity-recall-timeline.json)
- [GC-03 fail-closed](../../../fixtures/golden-cases/gc-03-activity-recall-fail-closed.json)

fixtureは、goalなし、multi-window focus、5分tick、excluded / idle gap、LM Studio障害、raw画像非永続化、`today / around / delete`を固定する。

## Founder N-of-1

### 比較

同じ日の同じ15分bucketから二つの表示をoffline生成する。

| 条件 | 入力 |
|---|---|
| R0 | FocusSegmentの時刻とapp ID |
| R1 | R0＋ActivityObservation |

時刻とdurationは両条件で同じにし、画像が活動内容の想起へ加える価値だけを見る。

### 最小試験

- 3日
- eligible frame 30枚以上
- 無作為に選んだ15分bucketを24件評価
- そのうち6件で「この時刻に何をしていたか」を後から検索

### Go

- goal、mark、window個別許可なしで3日使える。
- 6件中5件以上を`around`開始から30秒以内に正しく思い出せる。
- model claim precisionが90%以上。
- 3日のうち2日以上で、R1がR0にない有用な具体性を一件以上追加する。
- 訂正または確認時間が2分/日以下。
- excluded、locked、paused windowの画像送信、raw image disk write、body logが0件。

### Stopまたは縮小

- app denylistとpauseを頻繁に管理しないと安全に使えない。
- R1がR0より具体性を加えない。
- 誤ったactivityで記憶を上書きする、またはclaim precisionが90%未満。
- timeline確認が反芻、羞恥、自己監視、長時間の履歴閲覧を増やす。
- retention期限または`delete`でderived dataまで削除できない。
- x870障害がwindow timeline閲覧を止める。

R1がStopならscreen captureとLM Studioを外し、window metadataだけのR0を残す。R0自体に価値がなければcollectorも止める。

## 実装順

### P0: contract

1. GC-02、GC-03、ActivityObservation JSON Schemaをfixture testにする。
2. x870で一枚の`image_url + json_schema` smoke testを通す。
3. synthetic secretがLM Studioのhistory / logへ残らないことを監査する。

### P1: metadata timeline

1. `core / source-niri / store / app / cli`で`watch`とFocusSegmentを作る。
2. metadataだけで`today`と`around`を実装する。
3. lock、idle、pause、excluded、retention、deleteをtestする。

### P2: sparse visual observation

1. `capture-api / capture-niri`で5分tickのforeground captureを作る。
2. `model-api / model-openai`でActivityObservationを生成する。
3. failure時の破棄、no retry、late result無視をtestする。

### P3: 3日experiment

1. R0 / R1を同じbucketから生成する。
2. 24 bucketと6件の時刻検索を評価する。
3. Goならsystemd user serviceのenable flowを固定する。
4. Stopならvisual pathまたはcollector全体を削る。

## Non-goals

- required goal、task、要件、manual session
- Codex / terminal command wrapper
- window切替から集中、脱線、生産性を評価
- video、ambient audio、全monitor録画
- raw screenshot、OCR、prompt、agent transcriptの長期検索
- semantic search、embedding、vector database
- 自動task生成、自動schedule、通知
- Reflection、Resume、Return Anchor
- Screenpipe fork、Tauri、cross-platform

audioは恒久的な否定ではない。[11](11-qwen-audio-agent-assessment.md)のgateを満たした場合だけ、常時録音ではなく任意のpush-to-talk voice bookmarkまたはread-only voice queryとして検証する。

最初の勝ち筋は、完全なlifelogを作ることではない。

> 入力を増やさず、疎な画面観測と正確な時刻から、今日いつ何をしていたかを思い出せるかを検証する。
