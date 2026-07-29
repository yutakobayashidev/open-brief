# 09. Resume CueとWindow Transitionを比較するMVP

## Status

- 調査日: 2026-07-29
- 対象: terminal / Vim / AI coding agentを中心に、browser、Obsidian、chat、別terminalを行き来する作業
- 実行環境: Linux、Wayland、niri 26.04
- 位置づけ: 実装前の製品仮説とFounder N-of-1計画
- 判断: **MVPの介入は一枚のResume Cueにする。Window Transitionは最初に比較する受動inputであり、Task Switchまたは脱線の判定には使わない**

## 結論

最初に作るものは、ADHD向けScreenpipeでも、Codex hook viewerでもありません。

> AI coding中に文脈を見失ったとき、検索や履歴巡回をせず、一つのcommandで「戻るtask、止まった場所、次の一手、再開command」を一枚かつ10秒以内で取り戻せる、session-scopedなAttention Handoff CLIを検証する。

構成は次の順序にします。

```text
本人が置いた意図 + agentの検証可能な状態
                       ↓
        一枚のdeterministic Resume Cue
                       ↑
      比較条件としてforeground window遷移
                       ↑
        必要性が確認された場合だけ境界画像
```

画面取得は候補から外しません。ただし最初から常時録画、OCR、検索indexを作らず、`window遷移だけでは再開点が分からなかった`sessionで追加価値を比較します。rawなwindow遷移列も既定では表示しません。取得するsignalと、本人へ返す介入を分けます。

## 今回の修正

[07 Context Resumption](07-adhd-context-resumption-oracle-review.md)は、最小観測としてactive applicationと中断直前のapp switchを挙げながら、実装caseをIDEと会議へ寄せすぎていました。また、screenshotをMVPから一律に外していました。

今回の修正は次の4点です。

1. 初期対象を、IDEではなくterminal / Vim / AI coding agent利用者へ合わせる。
2. 一枚のterminal画像ではなく、terminal、browser、docs、chat、別terminalをまたぐ遷移列を主signalにする。
3. screenshotを全面採用または全面除外せず、境界keyframeの追加価値を独立して検証する。
4. Window Transitionそのものを製品価値とせず、一枚のResume Cueを改善するかという比較変数にする。

## OpenBrief内での位置づけ

このCLIはOpenBrief全体を一度に作る案ではない。[製品モデル](02-product-model-and-hypotheses.md)の`Continuity / Return`と仮説H3だけを切り出すvertical sliceである。Gmail / RSSの`Protect`、Slackへの`Signal`、有限brief、Curiosity Capture、calendar連携は今回のMVPへ入れない。

Resume laneのH3がFounder N-of-1でも成立しなければ、Resume用screen captureや複数source統合へ進む理由はない。成立した場合にだけ、同じ`openbrief-app`へ他のAttention Transition adapterを接続する。

一方、入力なしで「今日いつ何をしていたか」を返す[10 Activity Recall Timeline MVP](10-activity-recall-timeline-mvp.md)は、時間盲に対する別の仮説として先行検証する。両者はwindow eventの観測基盤を共有できるが、本書のResume効果とActivity Recallの想起価値を一つの指標へ混ぜない。本書で画像をM2まで待つ判断は、**Resume Cueへ画像を足すlaneだけ**に適用する。

ADHD特化はcapture技術ではなく、interactionの制約に置く。

- 中断前の入力を任意かつ一行以内にする。
- 復帰時には一枚、次の一手は一つだけ返す。
- Window切替を集中、脱線、人格の評価へ使わない。
- 同じ介入を全員へ固定せず、本人内で条件を比較する。
- raw dataをsession単位で閉じ、簡単に破棄できる。

## 根拠から言えること

### 1. ADHD研究は「全部記録すること」を支持していない

成人ADHDの複雑な展望記憶課題では、計画能力に大きな差が確認された一方、plan recall、self-initiation、executionの差は小さかった。この研究から直接screen captureは導けないが、「覚えていない」だけでなく「実行可能な計画へ変換できていない」可能性を分ける必要がある。

- [Fuermaier et al., Complex Prospective Memory in Adults with ADHD](https://pmc.ncbi.nlm.nih.gov/articles/PMC3590133/)

2026年の成人ADHD向けdigital health technology scoping reviewは133研究を同定したが、研究は治療、診断、clinical managementへ集中していた。日常のself-managementやcognitive assistive technologyは少なく、adherence、長期利用、当事者参加にも課題が残る。

- [Schofield et al., Digital health technologies for adults with ADHD](https://www.frontiersin.org/journals/digital-health/articles/10.3389/fdgth.2026.1746732/full)

成人ADHD向けcognitive assistive technologyの小規模研究では、weekly scheduleやwatchなど低技術の道具も高く評価され、個別調整と構造化された支援が必要とされた。高機能な自動captureが単純な外部手掛かりを上回るとは仮定しない。

- [Lindstedt and Umb-Carlsson, Cognitive assistive technology and professional support](https://pubmed.ncbi.nlm.nih.gov/23992459/)

### 2. 中断後の復帰には外部cueが役立つ

一般集団の実験では、中断前にprimary taskのcueを利用できると、復帰までのlagを短くできることが示されている。読書課題でも、中断位置を示すvisual cueが復帰を助けた。これはADHD固有の効果証拠ではないが、再開位置を環境へ残す設計を支持する隣接研究である。

- [Altmann and Trafton, Task Interruption: Resumption Lag and the Role of Cues](https://gregtrafton.com/papers/task_interruption.pdf)
- [Cane et al., The time-course of recovery from interruption during reading](https://doi.org/10.1080/17470218.2012.656666)

33実験、49介入を対象にしたmeta-analysisでも、中断対策は全体としてprimary taskの正確性を改善し、resumption lagを短縮した。ただし大半は実験室研究で、効果は介入とtaskの種類によって異なる。したがって「復帰cue」という介入仮説には根拠があるが、OpenBriefのwindow traceまたは画像が効くとはまだ言えない。

- [Guo et al., Effects of interventions to reduce the negative consequences of interruptions](https://pubmed.ncbi.nlm.nih.gov/34273814/)

### 3. 一つのtaskは複数windowにまたがる

27人を2週間観測したcomputing taskのfield studyでは、alertから別appへ移った後、さらに複数appへ連鎖することがあった。中断されたwindowが見えている場合は復帰が速く、参加者はcursorやhighlightを再開cueとして利用していた。著者らは、個別windowだけでなく、文書、表計算、分析softwareなどを含む広いtask contextの保存を提案している。

- [Iqbal and Horvitz, Disruption and Recovery of Computing Tasks](https://www.microsoft.com/en-us/research/wp-content/uploads/2016/11/CHI_2007_Iqbal_Horvitz-1.pdf)

したがって、次を同一視しない。

```text
Ghostty → Firefox → Obsidian → Ghostty
```

これは4回のtask switchではなく、一つの調査または実装taskのworking setかもしれない。

```text
Ghostty(open-brief) → Firefox(research) → Discord → Ghostty(dotnix)
```

この列は別taskへの移動を含む可能性があるが、metadataだけで`脱線`とは断定できない。

2026年にADHDの大学生21人から約180時間のPC activityを集めた探索的研究でも、window switching回数は自己報告のattention、effort、motivationと関連しなかった。小標本でstudy taskに限られるが、少なくとも`切替回数が多い = 集中していない`というscoreを作る根拠にはならない。

- [Towards Ecological Validity When Assessing ADHD Symptoms](https://pmc.ncbi.nlm.nih.gov/articles/PMC13095149/)

### 4. 開発者の復帰にはchronological cueとcross-app artifactが有望

371人のdeveloper surveyとlab studyでは、programmerは複数媒体のnoteへ依存していた。自動生成したcueを使った条件では、noteだけの条件よりtask完了成功率が約2倍で、参加者はcode snippetを時系列で示すcueを強く好んだ。

- [DeLine and Parnin, Evaluating Cues for Resuming Interrupted Programming Tasks](https://www.microsoft.com/en-us/research/publication/evaluating-cues-for-resuming-interrupted-programming-tasks/)

10,000 programming session、85人の観測では、1分以内にcodingを再開したsessionは10%だけで、再開前に別の場所を辿らなかったsessionは7%だけだった。

- [Parnin and Rugaber, Resumption Strategies for Interrupted Programming Tasks](https://sites.cc.gatech.edu/reverse/repository/resumptionstrategies.pdf)

2026年のTaskSnap研究は、code、website、documentをtask snapshotとしてまとめるsemi-automatedな方法を55人のdeveloperで評価した。TaskSnap条件では最初のcode editまでの時間が短くなった。一方、同論文はfully automatedなartifact groupingにはfalse positiveと情報過多があると整理している。

- [de Souza et al., TaskSnap: One Task at a Time With Snapshots](https://hasel.dev/publication/tasksnap-one-task-at-a-time-with-snapshots/)

OpenBriefへの含意は、IDEを作ることではない。AI coding時のtask artifactは、agent terminal、browser、docs、別terminal、repository、agent sessionへ分散するため、OS levelのwindow traceとagent-native eventを接続する価値がある。

### 5. 外部化には利益とcostの両方がある

intention offloadingのreviewでは、external reminderは将来意図の実行に有効だが、人がいつreminderを使うかにはmetacognitive biasがあると整理されている。2026年の実験では、reminderを外した後に、以前offloadしていた意図の成績がbaselineより低下した。

- [Gilbert et al., Outsourcing Memory to External Tools](https://pmc.ncbi.nlm.nih.gov/articles/PMC9971128/)
- [Fellers and Storm, Offloading reduces prospective memory learning](https://pubmed.ncbi.nlm.nih.gov/42241083/)

二つの事前登録実験では、reminderを置く身体的な手間を増やすと利用が減り、memory loadを補う効果も弱くなった。`毎回ちゃんと一行メモを書く`だけを完成形とせず、低costな自動下書きと比較する理由になる。

- [Chiu and Gilbert, Influence of the physical effort of reminder-setting](https://pubmed.ncbi.nlm.nih.gov/37642279/)

OpenBriefは認知能力の訓練を目的にしないが、依存costを無視しない。raw履歴を検索しないと何もできない製品ではなく、一時的なhandoffを本人のtaskへ返したら消せる道具にする。

## awesome-adhdとの整合

`awesome-adhd`の41 concept、6 entity、5 query、1 comparison、48 paper note、66 article noteを横断した[08 awesome-adhd synthesis](08-awesome-adhd-cross-report-synthesis.md)を再監査し、次のページをMVP判断へ直接使った。

- `concepts/task-resumption.md`
- `concepts/digital-interruptions.md`
- `concepts/prospective-memory.md`
- `concepts/external-memory.md`
- `concepts/working-memory.md`
- `concepts/passive-memory-assistants-adhd.md`
- `concepts/cognitive-personal-informatics.md`
- `queries/toymaker-passive-memory-adhd-design-2026.md`

横断して一致する原則は次である。

- 中断前の位置と次の一手を外へ置く。
- 記録量ではなく、再開に必要な最小情報を返す。
- 自己報告だけでなく、実際の復帰行動を見る。
- 同じ支援を全員へ固定せず、個人内で比較する。
- capture、通知、dashboardを新しい管理taskにしない。
- 行動dataを本人側に閉じ、監視や人格評価へ使わない。

ただし、41 conceptのconfidenceはhigh 1件、medium 25件、low 15件である。task resumptionとpassive memoryもmediumであり、`awesome-adhd`は仮説mapとしては有用だが、OpenBrief固有の効果証拠ではない。

## Evidence ladder

| 仮説 | 現時点の確からしさ | MVPでの扱い |
|---|---|---|
| 短い外部cueは中断後の復帰を助ける | 中。一般集団のmeta-analysisと実験が中心 | 中核介入として検証する |
| 開発taskのcontextは複数appにまたがる | 中。field studyとdeveloper研究が一致 | working setを単一windowへ限定しない |
| Window Transitionを加えるとADHD当事者の復帰が改善する | 低。直接研究なし | M0対M1で反証可能にする |
| Window切替回数から集中や脱線を判定できる | 支持なし。ADHD学生の探索研究では関連なし | scoreも通知も作らない |
| Resume Cueの境界画像がmetadataより追加改善する | 低。空間cueの隣接研究のみ | Resume laneではM1 failure後のM2に隔離する |
| 常時録画、OCR、長期検索が必要 | 支持なし | MVPから除外する |

## Oracle反証レビューの扱い

[07 Context Resumption](07-adhd-context-resumption-oracle-review.md)のOracleレビューでは、自動Resume Packを、deep linkだけ、または本人が5〜10秒で残す一行Return Anchorと比較するよう指摘された。自動packが復帰率、復帰時間、入力負担のいずれも改善しないなら、受動captureを止めるという反証条件も採用した。

2026-07-29に、terminal / Vim、niriのWindow Transition、TaskSnap、ADHD PC activity研究を追加したfollow-upを同じOracle会話で試みた。しかしOracleが既存ChatGPT会話のChat / Work modeを安全に判定できず、model実行前に終了した。この失敗を新しい第二モデルの支持として数えない。

したがって今回の更新は、一次研究、既存製品、`awesome-adhd`の横断結果と、前回Oracleのbaseline要求が一致する範囲だけを採用している。

## 既存製品との境界

| 製品群 | 主signal | 主に返すもの | OpenBriefで作らないもの |
|---|---|---|---|
| Screenpipe | screen、audio、app / window event | 過去の検索、timeline、AI回答 | 汎用の長期lifelog |
| ActivityWatch、RescueTime、Rize | app、window、URL、dwell | 時間分析、timesheet、focus評価 | productivity scoreと監視dashboard |
| Sunsama、Motion | task、calendar、deadline | 計画上の次の行動 | 自動schedule |
| OpenBrief Resume lane | bounded session、agent event、window transition | 中断直前の意図と今の最小一手 | 長期lifelog |

ActivityWatch、RescueTime、Rizeがwindow metadataを使っていることは、安価な観測signalとしての実用性を示す。ただしOpenBriefの復帰支援またはADHDへの効果証拠ではない。

- [ActivityWatch watchers](https://docs.activitywatch.net/en/latest/watchers.html)
- [RescueTime: How tracking works](https://help.rescuetime.com/article/245-how-rescuetime-works)
- [Rize automatic time tracking](https://rize.io/features/automatic-time-tracking)

## MVP候補の比較

| 候補 | 返すもの | 強み | 最大の弱み | 判断 |
|---|---|---|---|---|
| B0 Native Resume | cwd、agent session、既存のresume command | 追加製品なし | 元の意図と次の一手を返さない | 無介入baseline |
| M0 Manual Anchor | 一行の目的、次の一手、resume command | 最小、privacy costが低い | 不意の中断前には残せない | 必須baseline |
| M1 Trace-assisted Cue | M0＋agent event＋window traceから作る一枚のcue | 不意の中断でも最近のcontext候補を低costで返せる | window列から意図は一意に分からない | **検証する製品MVP** |
| M2 Visual Resume Cue | M1＋境界keyframe一枚 | visible stateと空間cueを返せる | capture、誤推定、privacy、実装cost | M1後の定性spike |
| M3 Searchable Lifelog | 長期screen/audio履歴と検索 | 広い過去を検索できる | 問題よりplatformが大きい | No-Go |

実装はM0から始めるが、製品として検証する最小案はM1である。意図は本人またはagent eventを優先し、window traceは作業位置の候補を補う。M1はraw traceを見せるtimelineではない。

## MVPの研究質問

最初の問いは一つにする。

> 一行Anchorとagent eventに、直近のcross-window遷移から抽出したcontext候補を表示すると、terminal-centricなAI coding sessionへの実質的な復帰が速く、正確になるか。

画面画像の問いは二番目に分ける。

> visible stateが必要だと事前に定義したsessionで、境界keyframeを一枚加えると追加改善があるか。

この順番なら、画像が必要か、metadataで十分かを混同しない。

## Golden Case: GC-HT-01

### 行動条件

- VimまたはterminalとAI coding agentを使う。
- 同一task中にbrowser、Obsidian、docsを頻繁に見る。
- 別project、chat、会議へ移った後、元のsessionで何を決めようとしていたか見失うことがある。
- IDE extensionは使わない。

### Session

```text
10:00  openbrief run --goal "Attention HandoffのMVPを決める" -- codex

10:03  Ghostty/open-brief
10:06  Firefox/TaskSnap paper
10:12  Obsidian/awesome-adhd
10:17  Ghostty/open-brief

10:20  Discord
10:23  Ghostty/dotnix

10:51  Ghostty/dotnixから openbrief resume --latest
10:52  Cueを読んでGhostty/open-briefへ戻る
```

期待する表示は、dashboard、長い要約、全遷移列ではない。

```text
Resume: Attention HandoffのMVPを決める

戻る
  Ghostty/open-brief

最近のcontext候補（未確認）
  Firefox/TaskSnap → Obsidian/awesome-adhd

最後に確認できたagent状態
  window transitionをtask switchと同一視しない

次
  docs/09のMVP比較を確定する

再開
  codex resume <session-id>
```

`最近のcontext候補`はAIが自動でtask分類した結果ではない。明示的に開始したsessionと、handoff境界前のfocus segmentから作る未確認候補である。

Discordや別projectへ移った経路は、実験時のreturn latency算出と誤推定調査には使うが、既定のResume Cueには表示しない。復帰に必要だと複数episodeで確認された場合だけ表示を再検討する。

### 受け入れ条件

- 同じtask内のterminal、browser、docs移動を`脱線`と表示しない。
- app titleのanimationやspinnerをfocus switchとして数えない。
- packを10秒以内に読める。
- 1つのanchor window、最大2つのcontext候補、1つのgoal、1つの次行動だけを表示する。
- `codex resume`など公開されたagent interfaceだけを使う。
- sensitive appは`ExcludedWindow`とだけ表示し、titleと画像を残さない。
- LLMがなくても同じ基本flowが動く。
- `今は戻らない`とraw data削除を一操作で選べる。

## CLIの最小surface

```text
openbrief run [--goal <text>] -- <agent-command>
openbrief mark [--next <text>]
openbrief resume [--latest]
openbrief discard [--latest] [--reason wrong|missing|privacy]
```

### `run`

- agent processの親として動く。
- 開始時のOpenBrief session ID、cwd、focused window IDをanchor identityとして固定する。
- niri event streamとagent hookを同じsessionへ束ねる。
- session終了時にobserverも終了する。
- 最初は同時に一つの観測sessionだけを許可する。

これによりalways-on daemon、全日timeline、session自動分類を避ける。

### `mark`

- 予測できる中断前には、直近のcontext候補をhandoffとして固定する。
- 不意の中断後に使った場合は、rolling metadataから直前の候補を作る。
- `--next`は任意にし、入力しなかったことをfailureにしない。
- niri keybindから呼べるone-shot commandにする。

### `resume`

- goal、1 anchor window、最大2 context候補、agent state、次の一手、resume commandを一枚で表示する。
- rawなWindow Transition、滞在時間、集中scoreを表示しない。
- 自動でwindowを開閉、移動、終了しない。
- Packを見たこと自体を成功に数えない。

### `discard`

- raw trace、keyframe、生成候補を削除する。
- 任意の`--reason`だけをtitleなしの実験値として残せる。
- confirmedな一行Anchorを残す場合も本人が選ぶ。

## Context候補の抽出仮説

MVPはLLMにtask境界を推定させない。また、次のruleを研究から証明された方法として扱わない。

1. `run`開始時のOpenBrief session ID、cwd、focused terminal window IDをanchor identityとして保持する。
2. `anchor → browser / docs → anchor`のように同じanchorへ戻った区間を`closed excursion`候補にする。
3. anchorへ戻る前に中断された区間も`open excursion`候補として捨てない。
4. `mark`または`resume`を境界にし、allowlist対象のfocus segmentからanchor以外を最大2件まで候補化する。
5. goalとnext actionは本人の`--goal` / `--next`を最優先し、なければ検証可能なagent stateだけを表示する。意図を補完して作らない。

Phase Aではclosed / openの両候補を本人の正解labelと比較し、どちらをM1へ出すかを決める。少なくとも`precision@2`、必要artifactの`coverage@2`、sensitive false positive、open-excursion欠落率を測る。十分な精度がなければ、M1は作らずM0で止める。

このheuristicには既知の破綻例がある。

- `terminal → paper → 不意の中断`では、必要なpaperがopen excursionに残る。
- 同じterminal windowでcwdやtmux paneが変わると、window IDだけでは別taskを区別できない。
- 同一taskでterminal Aからterminal Bへ移ると、同じanchorへ戻らない。
- browser内のURL遷移、terminal内のpane遷移はWindow Transitionへ現れない。
- anchorへの一瞬のAlt-Tabだけで、無関係な区間がclosedになる。

したがって`working set`とは呼ばず、未確認の`recent context candidates`と表示する。window IDだけで確定せずsession IDとcwdを併用し、曖昧なら候補を出さない。M1失敗時は、Window Transitionの仮説と抽出heuristicの失敗を分けて記録する。

## Event model

最初に必要なdomain eventは少ない。

```rust
enum Observation {
    SessionStarted {
        at: Timestamp,
        session_id: SessionId,
        anchor_window: WindowRef,
        cwd: PathBuf,
        goal: Option<String>,
    },
    FocusSegmentStarted {
        at: Timestamp,
        window: WindowRef,
    },
    FocusSegmentEnded {
        at: Timestamp,
        window: WindowRef,
    },
    AgentEvent {
        at: Timestamp,
        session_id: AgentSessionId,
        cwd: PathBuf,
        kind: AgentEventKind,
    },
    HandoffMarked {
        at: Timestamp,
        session_id: SessionId,
        next_action: Option<String>,
    },
}

enum StudyEvent {
    ConditionAssigned { at: Timestamp, episode_id: EpisodeId, condition: Condition },
    CueRendered { at: Timestamp, episode_id: EpisodeId },
    CueDismissed { at: Timestamp, episode_id: EpisodeId },
    ReturnObserved { at: Timestamp, episode_id: EpisodeId, session_id: SessionId },
    OutcomeValidated {
        at: Timestamp,
        episode_id: EpisodeId,
        correct: bool,
        mental_effort: Option<u8>,
        annoyance: Option<u8>,
        surveillance_discomfort: Option<u8>,
    },
}
```

`WindowRef`は、app ID、session内だけ有効なopaque window ID、workspace、policy適用後のtitleを持つ。`distracted`、`focused`、`productive`のような推定labelは持たない。

titleは既定で保存せず、allowlistしたappだけ保持する。raw traceはResume Cue生成または計測値抽出後に削除し、未処理でも24時間を上限にする。長く残せるのは、titleを除いたcondition、return latency、正誤、拒否理由などの実験値だけにする。

### niri eventの正規化

この環境のniri 26.04で次を実機確認した。

- `niri msg --json event-stream`は初期stateと更新をJSONLでstreamする。
- window ID、title、app ID、PID、workspace、focus状態、focus timestampを取得できる。
- terminal titleのspinnerだけでも`WindowOpenedOrChanged`が連続発火する。
- `set-dynamic-cast-window`、`set-dynamic-cast-monitor`、`clear-dynamic-cast-target`を利用できる。
- `screenshot-window --id <ID> --path <PATH>`で特定windowを取得できる。

したがってadapterはevent数をそのままfocus switch数にしない。

```text
raw WindowOpenedOrChanged
        ↓ state diff
focused window IDが変わったか
        ↓
FocusSegmentStarted / Ended
```

titleはmetadataとして更新するが、750ms程度debounceし、spinner変化をsession eventへ増幅しない。niri IPCのJSONは既存fieldとvariantを維持する方針だが、新しいfieldとvariantは追加されるため、unknown fieldを拒否しない。

- [niri IPC documentation](https://github.com/niri-wm/niri/wiki/IPC)
- [niri v26.04 release notes](https://github.com/niri-wm/niri/discussions/3899)

## 画面取得の位置づけ

Window Transition metadataは「どこを通ったか」を返せる。画像は「そのwindowで何が見えていたか」を返せる。どちらも「なぜ」を確定できない。この節はResume Cueへ画像を追加するM2を扱う。時間盲向けの入力不要な疎captureは[10](10-activity-recall-timeline-mvp.md)で別に定義する。

M2を試す場合も次に制限する。

- `--frames`を明示したsessionだけ。
- allowlistしたappだけ。
- stable focus後の低頻度keyframeをringへ置く。
- handoffあたり最大1枚。
- OCR、audio、全文検索を行わない。
- Pack確認後または24時間以内に削除する。
- modelへ送る場合は別の明示操作を要求する。

niriの`screenshot-window`はwindow IDを指定でき、capture spikeには使える。ただし自動実行するとclipboardも変更するため、常用backendにはそのまま採用しない。Dynamic CastとPipeWireを使うか、clipboardを変更しないwindow capture APIを別途検証する。

Screenpipeも2026年時点ではapp switch、window change、content changeをevent-driven captureのtriggerとしている。これは技術的な実現可能性の根拠であり、ADHDへの効果証拠ではない。

- [Screenpipe: event-driven screen capture](https://screenpipe.com/about)

## Crate境界

microcrate化は再利用境界を守るために行い、crate数をMVPの成果にしない。最初のvertical sliceは5 crateにする。

```text
openbrief-core       IDs、Observation、ResumeCue、policy
openbrief-source     source contract、niri / Codex adapter module
openbrief-store      session JSONLと短期artifact
openbrief-app        run、mark、resume、discard
openbrief-cli        clap entrypointと表示
```

依存方向は`cli → app → core / source / store`とし、OS固有型をcoreから参照しない。

二つ目のOS sourceまたはagent sourceを足す時点で、`openbrief-source`を`source-api`、`source-niri`、`source-codex`へ分ける。独立利用者がいないうちからcontractと実装を別publish unitにせず、module境界とfixture testで分離可能性を保つ。

画像の追加価値が確認された場合だけ、次を足す。

```text
openbrief-capture-api
openbrief-capture-niri
```

Tauriは`openbrief-app`を呼ぶadapterとして後から追加し、最初のMVPへ入れない。

## Founder N-of-1

これはADHD一般への効果試験ではなく、このFounder、このworkflow、このmachineでのfeasibilityと投資判断である。最初の判断点は2週間に置くが、効果方向を見るには条件ごとの件数が揃うまで最長4週間へ延長する。

### Phase A: shadow observation

最短3日、10件のcontext-loss episodeが集まるまで行う。2週間で10件に届かなければ、頻度自体をStop signalにする。

- `run`中のfocus segmentとagent eventだけを取得する。
- 介入を表示せず、anchor identity、closed / open excursion候補、privacy負担を確認する。
- episode後に、本当に必要だったartifact、計画的か不意か、候補の正誤を本人がlabelする。
- `precision@2`、必要artifactの`coverage@2`、sensitive false positive、open-excursion欠落率を測る。
- B0として既存のagent resumeだけで戻る時間も記録する。

B0は比較可能なcue表示時点がないため記述的baselineに留める。このpilotで因果的に比較する問いは、M1のM0に対する追加価値だけである。Resume Cue自体がB0を上回るかは、必要なら別のB0 / M0 randomized studyに分ける。

### Phase B: M0対M1

eligible episodeをcondition割当前に固定し、本人のnext actionがあるか、計画的か不意かでblock randomizationする。交互割当は使わない。両条件で同じObservationを取得し、`resume`時の表示だけを変える。無視されたepisode、5分以内に戻らなかったepisodeも、割り当てた条件のfailureとして残す。

eligibleとするのは、観測中のsessionが一つあり、明示的な`mark`があるかanchor / agent eventから60秒以上離れ、まだ正しい復帰行動をしていない時点で`resume`を呼んだepisodeに限る。conditionはこの判定後、Cue生成前に割り当てる。技術的にCueを生成できなかったepisodeも割当条件のfailureとしてprimary outcomeに残し、同時にfeasibility failureの理由を付ける。

| 条件 | 表示 |
|---|---|
| M0 | goal、本人のnext action、最後のagent event、resume command |
| M1 | M0＋Phase Aで選んだruleによる最大2件のrecent context candidates |

最初の2週間で各条件の実現性を見る。効果方向を投資判断へ使うのは各条件12件以上に達した場合だけとし、不足時は最長4週間まで延長する。それでも不足すれば、低頻度を理由にM1を止める。

### M2は別のqualitative spike

M1で`visible stateがないため戻れなかった`episodeが3件以上出た場合だけ、allowlistした境界keyframe一枚を最大3 episodeで試す。この事後選択からM2の因果効果は主張しない。

M1とM2を本当に比較する場合は、visible stateが必要になりやすいepisodeを実験前に定義し、同じ期間内でM1 / M2をrandomizeする別studyにする。

## Metrics

Primary outcomeは`resume`呼び出しから、最初の`validated correct return`までの時間とし、5分で打ち切ったfailureも300秒として残す。

```text
対象OpenBrief sessionとcwdへ戻る
かつ
（同じagent sessionで次のturnが発生する
 または対象contextで行動を始める）
かつ
本人がepisode後に「正しい復帰だった」と一操作で確認する
```

診断用に二つの時計へ分ける。

- tool overhead: `resume`呼び出し → Cue表示
- intervention latency: Cue表示 → validated correct return

中断終了またはreturn opportunityから`resume`を呼ぶまでの時間は、明示的な`mark`など開始点を観測できるepisodeだけ記述的に扱う。CLIを呼ばなかったcontext-lossは、一日一回の短い振り返りでmissed episodeとして数え、成功例だけを残さない。

Secondaryとguardrailは次に絞る。

- 5分以内のvalidated correct return率
- `precision@2`と必要artifactの`coverage@2`
- cueの読了時間と無視率
- `wrong / missing / privacy`によるdiscard率
- mental effort、煩わしさ、監視感の任意1〜5評価
- titleまたは画像を残したくなかったepisode数

`app switch数`、`集中score`、`productivity score`は成功指標にしない。

## Continue / Stop gate

次は科学的thresholdではなく、個人開発の投資判断である。

### Phase AからM1へ進む

- context-lossが週3件以上あり、少なくとも10件をlabelできる。
- recent context candidatesの`coverage@2`が70%以上、`precision@2`が50%以上で、sensitive false positiveが0件。
- cue候補の訂正または確認が新しい管理taskにならない。

このgateを満たさなければM0で止め、Window Transitionを製品inputにしない。

### M1を続ける

- 各条件12件以上で、M1のprimary outcomeがM0より20%以上短い、または30秒以上短い方向を示す。
- 5分以内の正しい復帰率、mental effort、煩わしさがM0より悪化しない。
- cueのmedian読了が10秒以内。
- M1の本人選好はsecondaryとして記録し、単独のGo理由にしない。

### M1を縮小または止める

- 文脈喪失episodeが週3件未満で、導入costに見合わない。
- 一行Anchorとagent resume commandだけで十分。
- window候補の訂正または探索に、短縮した時間以上を使う。
- 最長4週間または各条件12件で、正しい復帰に改善方向がない。
- traceが新しいtimeline確認習慣になる。

### 件数に関係なく即時停止

- allowlist外のtitleまたは画像を永続保存または外部送信した。
- 別sessionへの危険な誤誘導が起きた。
- `discard`またはretention期限でraw dataを削除できなかった。

Resume laneでは、このgateを満たすまでScreenpipe fork、OCR、audio、長期検索、MCP、Tauri、cross-platform captureへ進まない。別laneのActivity Recall Probeは[10](10-activity-recall-timeline-mvp.md)のapp denylist、非永続化、R0 / R1 gateに従い、Activity RecallのGoをResume M2の証拠として数えない。

2026年のEUNETHYDIS consensusも、ADHD向けdigital toolには適切なclinical evidenceが不足しているとし、当事者中心の設計、厳格な評価、data collectionと限界の透明性、privacy、非digital支援を置き換えないことを求めている。OpenBriefは治療または症状検出を主張せず、このMVPを行動支援の個人内実験として扱う。

- [EUNETHYDIS Consensus Statement on Digital Health and ADHD](https://pmc.ncbi.nlm.nih.gov/articles/PMC13374793/)

## 最初の実装順

1. Golden Caseの一枚Resume Cueとcontext候補poolをfixture / snapshot testで固定する。
2. 5 crateのworkspaceで、M0のbounded `run`、`mark`、`resume`、`discard`とCodex公開hookを作る。
3. synthetic niri JSONL fixtureでfocus reducerとspinner event除外をtestし、Phase Aのshadow observerを作る。
4. 本人labelに対するcandidate ruleのprecision / coverage gateを通った場合だけM1へ足す。
5. M0 / M1のblock randomizationとmetricsを入れ、最短2週間使う。
6. visible state不足が3件以上だった場合だけ、因果比較ではないcapture spikeを行う。

最初の勝ち筋は、Screenpipeより多く記録することではない。

> 何をしていたか分からなくなった瞬間に、本人の認知負荷を増やさずに済むかも含め、元のsessionへ戻るための最小cueが役立つかを検証できるか。
