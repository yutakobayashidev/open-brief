# Design memo: OpenBrief as an Attention Control Plane

## Product definition

> OpenBriefは、過去の行動、外部から届く情報、本人の未整理な意図を根拠付きの注意候補として受け取り、いま扱う量へ有限化し、本人の判断と戻り先を特定のAgentに依存せず保持するlocal-firstなAttention Control Planeである。

OpenBriefは録画app、AI付きTodo、Agent clientのどれか一つではありません。それらを置き換えるのではなく、複数のproducerとAgentをまたいで本人の注意状態を保つ小さなauthorityです。

一番短い製品表現は次です。

> OpenBriefは、見落としをゼロにするappではなく、根拠を見ながら「今は見なくてよいもの」を本人が決めるappである。

AIは候補と理由を作ります。何を扱うか、扱わないか、どこへ戻るかを確定するのは本人です。

## 解くpain

情報が存在しないことより、次の受け渡しが失われることを問題とします。

- 気になるものが増え続け、今見る対象を有限化できない
- 中断後に、元の作業と次の一手を思い出せない
- Gmail、Slack、画面履歴、Agent会話へ判断と根拠が分散する
- 興味や不安までTodo化され、未処理リストが負債になる
- Agentを替えると、本人が決めたこととその理由まで失われる

これはADHDの診断や治療を目的としません。優先順位づけ、時間盲、中断後の復帰、情報過多、入力前の整理負荷といった、ADHDで生じやすい実行上のpainを支援対象にします。

## 3つの入力、2つのprojection

入力は同じ`Observation ingress`へ正規化しますが、意味を混ぜません。

| 入力 | 答える問い | 主な根拠 |
|---|---|---|
| Screen / app history | いつ、何をしていたか | 時刻、継続時間、切替、keyframe |
| Gmail / Slack / GitHub / News | 何へ対応・接触する価値があるか | 期限、待機者、影響、現在目標との関係 |
| Natural-language dump | 本人は何を気にし、何へ戻りたいか | 本人の言葉、迷い、現在意図 |

同じObservationから、用途の異なるprojectionを作ります。

```text
Screen / app history ───────────────→ Activity Recall
                                           │
External sources ─┐                        │
Natural dump ─────┼→ Attention Triage ─────┼→ User Decision
Recall evidence ──┘                        │       │
                                           └───────┴→ Return Anchor
```

### Activity Recall

時間盲に対して、過去の事実を復元します。

- 今日いつ何をしていたか
- どこで長く時間を使ったか
- 中断前にどのcontextへいたか

Activity Recallは生産性評価ではありません。window切替回数を集中度や脱線のscoreに変換せず、不明な時間を推測で埋めません。

### Attention Triage

情報過多に対して、今見る候補を有限化します。

- 今扱う
- 今は扱わない
- 好奇心として残す
- 判断材料が足りない
- 処理後にどこへ戻る

RecallはTriageの根拠に使えますが、「画面で見ていた」ことを「重要である」ことへ変換しません。

## 中心flow

```text
Observe
  ↓
Protect ── 見失いたくない意図を守る
  ↓
Signal  ── 必要なら応答状態だけを周囲へ伝える
  ↓
Explore / Focus
  ↓
Capture ─ 興味や気がかりを義務化せず残す
  ↓
Return  ─ 元の作業または次の具体的行動へ戻る
```

有限Briefはこのflowの入口です。Brief自体を新しいInboxや永続Todo listにしません。

## Control Planeが所有するもの

OpenBriefはAgentのchat historyを正本にせず、次をlocal authorityとして保持します。

| State | 意味 |
|---|---|
| Observation | source、時刻、根拠を持つ未確定の事実 |
| Brief proposal | Agentが作った最大3件の有限候補 |
| User Decision | 本人が確定した`今扱う / 扱わない / 保留` |
| Curiosity Capture | 義務にしない問いや興味 |
| Protected Intent | 見失いたくない作業や返信 |
| Return Anchor | 戻るcontextと次の物理的な一手 |

Agentの提案と本人の決定は別の型、別の永続状態です。Agentを終了または交換しても、本人の判断と戻り先は残ります。

```text
Producers                         Reasoners
Screen / niri ───────┐            Codex ACP
Hermes cron ─────────┤            Hermes / OpenClaw
Natural-language ────┤                   │
                     ▼                   ▼
              Observation ingress   proposal-only MCP
                     │                   │
                     └──────┬────────────┘
                            ▼
                   OpenBrief local authority
                   evidence / proposal / decision
                            │
                            ▼
                      Desktop / CLI
```

## Agentとmodelの役割

| Component | 責務 |
|---|---|
| Hermes / OpenClaw | source収集、tool利用、scheduled producer、必要なら外部操作 |
| ACP Agent | Desktop上のstatefulな対話、stream、cancel、認証 |
| MCP | AgentがOpenBriefを読み、Briefやtriageを提案する狭いaction plane |
| LM Studio / VLM | 画面keyframeを構造化Observationへ変換するstatelessな推論 |
| OpenBrief | provenance、privacy policy、本人の決定、Return Anchorのauthority |

有限Briefの生成と自然言語triageはAgentが担います。LM Studioは画面理解用VLMの境界であり、現在のBrief生成runtimeではありません。

## Briefの設計

Briefは順位表ではなく、本人が判断するための有限な比較面です。

- 今扱う候補は最大3件
- 各候補にsource、時刻、理由、根拠、不明点を付ける
- `今は扱わない`を肯定的な結果として扱う
- 興味、気がかり、義務、情報不足をすべてTodoへ変えない
- 本人確認なしにDecisionやReturn Anchorを確定しない

source横断の単一scoreは作りません。少なくとも次の理由を分けます。

```text
urgency             期限が近いか
consequence         放置した影響が大きいか
waiting_on_you      誰かの次の行動を止めているか
personal_relevance  本人の現在意図に関係するか
novelty             単に新しく面白いだけか
```

理由は順位より重要です。同じSlack messageでも返信待ちと雑談では意味が異なり、同じニュースでも現在の調査へ直結するものと単なる新奇性では扱いが異なります。

## Natural-language dumpの位置づけ

自然言語入力は新しいTodoを作るformではありません。整理前の状態をそのまま受け取り、機械が作ったBriefへ本人の現在意図を注入する補正入力です。

```text
今日は課題を終わらせたい。
Slackは緊急ではない。
調査書だけ気になる。
```

この入力から提案できる状態は次です。

```text
今扱う       調査書のメール
今は扱わない Slack、AIニュース
戻り先       数学レポートの次の設問
```

曖昧なdumpへtitle、期限、優先度の入力を要求しません。

## ADHD向けのinteraction原則

1. 増やすより減らす。最初の画面は最大3件と次の一手だけにする
2. 覚えさせず外在化する。元の意図と戻り先を常に同じ場所へ置く
3. 気がかりを義務化しない。Capture、保留、手放すを正常な終端にする
4. AIは提案者に留める。理由を示し、本人が短い自然言語で修正できる
5. 分析で終わらせない。確定後は一つの具体的な再開操作を提示する

成功は整理した件数ではなく、読む対象が減り、判断時間が短くなり、実際に元の作業へ戻れたかで測ります。

## Safetyとprivacy

- 絞り込みを元sourceからの削除や既読化と同一視しない
- 初期検証はshadow modeとし、重大な候補を自動で不可視化しない
- remote Agentへ送るObservationの範囲と送信先を表示する
- screen contentはdenylist、pause、retention、fail-closedを備える
- external writeは収集・提案経路と分離し、対象と変更内容を本人が確認する

「見なくてよい」を決めるほどfalse negativeの危険は高まります。Briefだけを信頼できず毎回Inboxを二重確認するなら、この設計は注意負荷を減らしていません。

## 非目標

- ADHDの診断、治療、症状全般の代替
- すべての情報を取り込み、見落としをゼロにすること
- AIが本人の重要度や行動を最終決定すること
- Activity historyから生産性scoreや監視指標を作ること
- Gmail、Slack、calendar、Todo serviceのsource of truthになること
- 一つのAgent、model、window managerへ本人の状態を閉じ込めること

## 製品仮説と評価

中核仮説は「AIの分類が賢い」ことではありません。

> 根拠付きの候補を最大3件へ有限化し、今扱わない選択とReturn Anchorを本人が確定できると、生のInboxや時系列履歴だけを見る場合より、注意負荷と復帰失敗を減らせるか。

主要な成功指標は次です。

| Outcome | Measure |
|---|---|
| 有限化 | Brief確定までの時間、表示後も元Inboxを確認した割合 |
| 削減 | `今は扱わない`を安心して確定できた件数 |
| 行動開始 | 確定から最初の具体的操作までの時間 |
| 復帰 | Return Anchorの作業へ実際に戻った割合と時間 |
| 信頼 | 重大なfalse negative、AI提案の修正率、privacy discomfort |

Activity RecallとAttention Triageは共有基盤を持ちますが、効果は別々に測ります。履歴が思い出しを助けたことと、Briefが選択を助けたことを一つのscoreへ混ぜません。

## 現在の実装との関係

- R0 Context Recallは、Activity Recall projectionのmetadata-only実験
- Desktop Agent MVPは、Observation → finite Brief → natural-language triage → user confirmation → Return Anchorのvertical slice
- screen captureとLM Studio VLMは、Activity Recall向けObservation producerの次段階
- Hermes cronは、Gmail / Slack等をObservationへ変換するscheduled producer

したがって、画面履歴と有限Briefは競合する製品案ではありません。異なるpainを解く二つのprojectionであり、本人の注意状態と戻り先を同じlocal authorityへ接続する点で一つの製品になります。
