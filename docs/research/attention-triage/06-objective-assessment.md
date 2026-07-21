# 06. ClawBrif構想の客観評価

## 結論

**研究用prototypeは続行する。独立SaaSへの本格投資は保留する。**

2026-07-21時点のClawBrifには、検証する価値のある研究仮説があります。一方、製品市場適合、重大メールの見落としを防ぐ安全性、継続利用、事業上の防御力は実証されていません。

```text
現在の判断

研究prototype     Go
local-first OSS   条件付きGo
独立B2C SaaS      Hold
大規模事業化       No-Goに近い
```

総合点をあえて1つにすると、現時点では **5.5 / 10前後** です。この点数は「弱いアイデア」という意味ではありません。問題は重要で、仮説も明確ですが、効果と事業性を示すClawBrif固有の証拠がまだない、という意味です。

次に作るべきものは統合基盤ではなく、3週間で中核仮説を壊せる比較実験です。

> 単純な有限batchと一行の復帰手掛かりを超える複雑さに、本当に追加価値があるか。

## 評価の前提

- 評価日: 2026-07-21
- 対象: 現在の研究文書、GC-01 fixture、Tiimo静的解析との比較
- E1: ClawBrif自体の実証結果はまだ存在しない
- 第二モデルレビュー: Oracle経由のGPT-5.6 Sol、Extra High reasoning
- 外部確認: 一次研究、公式製品ページ、Google公式要件

この評価は完全な意味で「客観的」ではありません。現在入手できる証拠に評価範囲を限定し、支持する材料と中止理由を同じ基準で並べたスナップショットです。

競合の機能は各社の公式説明に基づきます。利用者数、継続率、実際の効果までは独立に確認していません。Oracleの回答も結論ではなく、見落としを減らすための助言として扱います。

## 何を作ろうとしているか

ClawBrifの価値は、複数sourceの収集やAI要約そのものではありません。仮説の中心は、注意の遷移を次の順番で支援することです。

```text
義務を見失わない
    ↓
応答できない状態と復帰予定を周囲へ伝える
    ↓
好奇心を有限に探索する
    ↓
気になった問いをタスク化せず残す
    ↓
探索前の作業へ戻る
```

この流れを製品用語へ置き換えると、`Protect → Signal → Explore / Focus → Capture → Return`です。Signalは、集中や探索で応答できない状態と復帰予定を、本人が選んだ範囲で周囲へ出力します。

- Protect: Gmailなどから、今日見失いたくない意図だけを本人が確定する
- Signal: 応答状態と復帰予定を、本人が選んだ範囲で周囲へ伝える
- Explore / Focus: RSSなどを有限のtopicとして読む、または集中作業を行う
- Capture: 気になった問いを新しい義務へ変えず退避する
- Return: 探索前のcontextと次の一手を再提示する

個々の仕組みには近接研究があります。

- 通知のbatchingは、通知の割り込み方とwell-beingの関係を変え得る
- 明示的な終端やstop cueは、無限feedより利用終了を助け得る
- 中断前のresumption cueは、作業再開を助け得る

ただし、それらを統合したClawBrifの効果は未検証です。したがって、現段階で主張できるのは「効果がある」ではなく「安価に反証でき、検証する意味がある」です。

## スコアカード

10点は強い証拠があり危険が低い状態、5点はもっともらしいが未検証の状態を表します。

| 観点 | 点数 | 判断理由 |
|---|---:|---|
| 問題の重要性 | 8 / 10 | 義務の見落とし、探索超過、作業復帰は明確で切実な問題 |
| 概念上の差別化 | 6 / 10 | CaptureをTodo化せず、Returnまで一連に扱う点は特徴がある |
| 研究上の新規性 | 6 / 10 | 個別要素は既知だが、注意遷移protocolとしての統合は検証余地がある |
| 構成概念妥当性 | 4 / 10 | 現行指標では「戻ると選んだ」と「実作業へ復帰した」が混ざる |
| prototype実現性 | 8 / 10 | Wizard-of-Ozとlocal dataだけで中核仮説を試せる |
| 複数sourceの製品化 | 5 / 10 | adapter、認証、同期失敗、source差を扱う必要がある |
| 採用と信頼 | 4 / 10 | 重要情報を隠す製品には、一度の重大な見落としでも不信が生じる |
| 防御力 | 3 / 10 | 要約、prompt、adapter、有限feedは模倣されやすい |
| 製品市場適合 | 4 / 10 | 強い個人的painはあるが、他者の継続利用と支払意思は未確認 |
| 安全性・privacy準備 | 5 / 10 | read-only、人間確認は良いが、運用・監査設計は未完成 |

用途別の判断は次のとおりです。

| 位置づけ | 評価 | 理由 |
|---|---:|---|
| HCI研究テーマ | 7 / 10 | 反証可能な問いと測定対象を作れる |
| 個人用local-first OSS | 8 / 10 | privacyとdistributionの負担を抑えて本人のpainを解ける |
| 特定prosumer向けtool | 6 / 10 | 開発者・研究者などには行動patternが合う可能性がある |
| 独立B2C SaaS | 4 / 10 | trust、継続率、Gmail要件、支払意思が未検証 |
| VC型の大規模事業 | 3 / 10 | distributionと防御力の根拠が弱い |

## 最も強いBuild argument

最も強い続行理由は、**重要で、統合に意味があり、しかも安価に反証できる仮説**だからです。

普通の情報整理toolは「より多く集め、より効率よく読む」方向へ進みます。ClawBrifは、情報接触の前後まで扱います。

1. 探索前に守りたい意図を外在化する
2. 探索範囲に終端を作る
3. 着想を義務へ変えず保存する
4. 元のcontextへ戻る手掛かりを出す
5. 実際に戻れたかを測る

特に`Curiosity Capture`と`Return Anchor`の組み合わせは、単なるdigestとの差を作れる可能性があります。これが実測で効けば、研究上も製品上もClawBrifの中心になります。

## 最も強いStop argument

最も強い中止理由は、**価値を出すには情報を隠す必要がある一方、安全であるには重要な義務を隠してはならない**という緊張です。

ClawBrifは接触情報量を減らすほど便利になります。しかし、返信期限、請求、セキュリティ警告などを一度でも誤って隠すと、ユーザーは元のInboxを二重確認します。その時点で注意負荷が減らず、ClawBrifが新しいInboxとして増えます。

```text
絞り込みを弱める
    → 安全だが、元のInboxと差がない

絞り込みを強める
    → 便利だが、重大なfalse negativeが怖い
```

この問題を解かないままGmail API、複数adapter、LLM分類、calendar連携を作ると、高コストな統合基盤だけが先に完成します。

## 市場で既に提供されているもの

2026-07-21時点で、次の機能単体は差別化になりません。

- 複数sourceの収集
- AI要約
- daily brief
- 重要メールとTodoの抽出
- RSSや動画の有限digest
- 音声briefing
- アプリを開く前の摩擦や時間制限

公式ページ上では、近接製品が既に次を提供しています。

| 製品 | 近接機能 | ClawBrifへの含意 |
|---|---|---|
| Google Gemini Daily Brief | Gmail、Calendar、Gemini chatsの優先表示、返信や予定化の提案 | Gmail起点のdaily priorityだけでは差別化できない |
| DailyStack | Gmail、Outlook、Calendar、GitHub、Linear、Jira、Todoistなどのdigest | 多source統合そのものは既存category |
| Nudget | RSS、YouTube、X、podcast、newsletterなどのmorning briefing | 好奇心sourceの集約と要約は既に提供される |
| Shortwave | AIによるmail整理、重要mail、Todo、bundle、配信schedule | Gmail triageは成熟した競争領域 |
| one sec | app前のfriction、意図確認、制限、blocking | 強制せず注意遷移を支えるUXも近接領域がある |

したがって、ClawBrifが検証すべき差は次の3点です。

1. Curiosity CaptureをTodoへ変換しないこと
2. 探索前の作業contextを保存し、後で再提示すること
3. `return_chosen`と実際の作業復帰を分けて測ること

## 防御力の評価

現時点の競争上の防御力は弱いです。

- promptは模倣できる
- source adapterは競合も作れる
- topic化とsummaryはmodelの標準能力になる
- 有限feedはUIとして再現しやすい

将来、次のいずれかが実証されれば防御力になり得ます。

- ClawBrif固有protocolによる実測済みのReturn改善
- 個人ごとに、どの介入が効くかを学習するpolicy
- agentやCLIから安全に書き込めるadapter ecosystem
- local-first、read-only、根拠表示による信頼
- 「読む効率化」ではなく「注意遷移」というcategory framing

ただし、これらは現時点ではmoatではなく候補です。

## Top 5 kill risks

### 1. 重要な義務をAIが隠す

- 発生確率: 中〜高
- 重大度: 致命的
- 早期signal: 元Inboxの二重確認、保護漏れ、critical mailの見落とし
- 対応: 初期実験ではshadow modeにし、AIにmailを非表示にさせない

### 2. ClawBrif自体が新しいInboxや儀式になる

- 発生確率: 高
- 重大度: 高
- 早期signal: 未処理badge、Capture backlog、長い朝のtriage、罪悪感
- 対応: CaptureをTodo化しない、残件を義務表示しない、処理時間を測る

### 3. 単純な有限batchと復帰メモに勝てない

- 発生確率: 中〜高
- 重大度: 致命的
- 早期signal: topic化、AI理由、分類を足してもReturnや探索超過が改善しない
- 対応: content量を揃えた比較で、各機構の増分効果を測る

### 4. source、認証、privacyの費用が便益を超える

- 発生確率: 高
- 重大度: 高
- 早期signal: adapter保守に研究時間の大半を使う、OAuth審査が必要になる
- 対応: 最初はlocal import、manual export、Wizard-of-Ozで試す

### 5. 成功しても既存製品に吸収される

- 発生確率: 高
- 重大度: 高
- 早期signal: competitorがReturnやfinite briefingを標準機能として追加する
- 対応: 機能数ではなく、対象行動、実測効果、信頼modelに集中する

## 研究設計の評価

### 現在の位置づけ

- 研究上の新規性: 6 / 10
- 構成概念妥当性: 4 / 10
- 現在のfull paper readiness: 3 / 10
- 再設計とfield study後の可能性: 7 / 10

現在のN-of-1は、instrumentationと本人の行動patternを確認するpilotとして有用です。しかし、ClawBrifの有効性、ADHDへの効果、一般的な製品市場適合の証拠にはなりません。

### 直すべき交絡

#### 1. Return条件で複数要素を同時に変えない

Return AnchorとAnchor Checkを同じ条件だけへ入れると、どちらが効いたか分かりません。最初はReturn Anchorの有無だけを変えます。

#### 2. deep linkを全条件へ揃える

Anchor条件だけtargetへのlinkを持つと、復帰改善が記憶支援ではなくnavigation短縮で説明できます。全条件へ同じlinkを置き、保存contextの有無だけを変えます。

#### 3. 実作業への復帰を測る

`return_chosen`や最初のtarget操作だけでは、実質的な作業復帰とは言えません。

暫定的には次を満たした場合を`substantive return`とします。

```text
対象contextを開く
かつ
2分以上継続する、または意味のある編集・操作を行う
```

#### 4. AIが隠した対象も安全性評価へ含める

ユーザーがProtectしたmailだけを分母にすると、AIが候補へ出さなかった重要mailが測れません。独立したaudit用gold setを作り、action-required recallとhigh-loss false negativeを評価します。

#### 5. 時間を記録できないsessionを捨てない

測定不能sessionを除外すると、失敗しやすい条件だけが消える可能性があります。発生率を記録し、総時間とsession単位の結果を併記します。

#### 6. 24時間後の着想価値を過信しない

自己評価は期待や需要特性の影響を受けます。保存後に実際に参照、共有、試作、執筆へ使われたかも補助指標にします。

#### 7. 対象者を診断名で広げない

初期対象は、診断の有無ではなく次の行動条件で募集します。

> 情報を調べ始めると、直前の作業へ戻ることが難しいと自己報告する人。

RSS利用者は技術系prosumerへ偏ります。また、有限RSSだけの結果を無限SNSへ一般化できません。

## 最初に狙う利用者

最初から「ADHD向け生産性アプリ」として広く売るべきではありません。診断や治療効果を主張せず、行動patternで対象を絞る方が妥当です。

有望な初期対象は次です。

- 開発者
- 研究者
- security analyst
- creator
- GmailとRSSや技術newsを頻繁に使うprosumer
- 情報収集の価値を感じる一方、元作業への復帰に失敗しやすい人

初期形態は、企業による従業員監視型serviceではなく、個人用のlocal-first tool、browser extension、IDE extensionが適しています。

## 技術・privacy feasibility

local-firstなデータ境界、BYOM、remote送信条件は、[ADR 0001](../../adr/0001-adopt-local-first-data-and-model-boundaries.md)でAcceptedの技術判断として固定しています。Signal段階とSlack Status Outputの境界は、[ADR 0002](../../adr/0002-adopt-attention-signals-and-slack-status-output.md)で固定しています。

### prototype

中核仮説のprototypeは実現可能です。production用Gmail APIや常駐agentは不要です。

- mailとRSSを手動またはlocal fileで入力する
- 人間が裏で候補を作るWizard-of-Oz方式を使う
- 実験UIだけを作る
- sourceへの書き込みは行わない
- 全Observationをaudit可能に残す

### public Gmail integration

公開SaaSでrestricted Gmail dataをserver側へ保存、転送、処理する場合、OAuth verificationや年次security assessmentが必要になる可能性があります。

また、Gmail pushの`watch`は少なくとも7日ごとの更新が必要で、notificationの遅延や欠落も考慮しなければなりません。

したがって、Gmailは「小さなadapter」ではありません。personal use、test環境、local処理は初期検証を軽くしますが、そのままpublic distributionの解決にはなりません。

## 3週間の最小実験

### 今回は作らないもの

- production Gmail API連携
- 複数source adapter基盤
- LLMによる自動重要度分類
- Calendar書き込み
- 常駐agent運用
- 認証、課金、組織管理
- podcast、動画生成

### 実験デザイン

2×2のwithin-subject randomized designを使います。

| 要因 | 条件1 | 条件2 |
|---|---|---|
| presentation | 時系列の有限batch | topic化した有限brief |
| resumption | Return Anchorなし | 一行のReturn Anchorあり |

全条件で次を揃えます。

- 同じObservation pool
- 同じ情報量とおおむね同じ文字量
- 同じsource linkとdeep link
- 同じ残件数、見積時間、明示的終端
- 同じscratch note機能

Curiosity Captureの有無は最初の比較で変えません。presentationとresumptionの効果を測った後、必要なら独立した要因として追加します。

### 参加者とsession

- 最低8〜12人
- 1人10〜12 session
- 合計100〜120 sessionを目安にする
- 行動条件でscreeningする
- founder N-of-1は先にinstrumentation確認へ使う

### Primary outcome

10分以内の`substantive return`率をprimary outcomeにします。

```text
対象contextを開いた
かつ
2分以上作業を継続した、または意味のある編集・操作を行った
```

### Safety co-primary outcome

独立auditでaction-requiredと判断された義務に対する、criticalまたはhigh-loss false negativeを測ります。

### Secondary outcomes

- Return latency
- 予定時間を超えた探索率
- 後で実際に利用された着想の割合
- 自律性と納得感
- irritationと圧迫感
- native sourceへ脱線した回数
- setupと修正に要した時間
- 4週間継続したいか
- 少額でも実際に支払うか

## 暫定Continue gates

次は科学的に確立された閾値ではなく、過剰投資を防ぐための事前判断基準です。

以下を満たす場合、ClawBrif MVPへ進みます。

- Return率が15 percentage point以上改善する、またはmedian latencyが25%以上短くなる
- topic化により探索超過が20%以上減る
- 高価値の着想がcontrolの90%以上維持される
- seriousなcritical mail見落としが0件
- setupと修正のmedianが90秒以内
- 10人中4人以上が、さらに4週間使いたいと答える
- SaaSを主張するなら、10人中2人以上が少額を実際に支払うかdepositする

## Stop conditions

次のいずれかなら、現在の構想またはその一部を止めます。

- content量を揃えてもtopic化がfinite batchを上回らない
- Return Anchorを置いても実作業への復帰が改善しない
- Curiosity Captureが新しいbacklogと罪悪感を作る
- 重要mailのfalse negativeを許容範囲へ下げられない
- setup、修正、二重確認の時間が削減時間を上回る
- 参加者が実験後に継続利用を望まない
- privacy懸念によりreal dataでは使われない

## 結果別の判断

| 実験結果 | 判断 |
|---|---|
| topic化とReturnの両方が効く | ClawBrif MVPへ進む |
| Returnだけが効く | aggregatorを作らずbrowser / IDE Return extensionへ縮小する |
| finite batchだけが効く | 既存RSS・mail client設定で十分。独自aggregatorを止める |
| Curiosity Captureが負担になる | coreから外す |
| 効果はあるがprivacyで拒否される | local-first OSSまたはpluginへ限定する |
| 効果がなく継続希望もない | 研究と製品化を止める |

## 最終recommendation

ClawBrifは、現段階では「作るべき完成製品」ではなく、**試すべき注意遷移protocol**です。

実装順は次の1本に絞ります。

1. GC-01を使って2×2実験UIを作る
2. shadow modeで安全性指標を実装する
3. founder N-of-1で計測不良を直す
4. 8〜12人の行動screened participantで試す
5. 事前に決めたContinue / Stop条件で判断する

ここで効果が出るまで、adapter数、podcast生成、calendar連携、public Gmail OAuthへ投資しません。

名称も、効果が実証されるまでは保証を連想させる`Attention Firewall`より、`Attention Transition`または`Brief and Return`の方が正確です。

## 参考文献・公式情報

### 近接研究

- Fitz et al. (2019), [Batching smartphone notifications can improve well-being](https://doi.org/10.1016/j.chb.2019.07.016)
- Baughan et al. (2022), [How explicit stopping cues affect digital media use](https://doi.org/10.1145/3491102.3501899)
- Ratwani and Trafton (2008), [The role of resumption cues in interrupted task performance](https://doi.org/10.1080/13506280802025791)

### 近接製品

- [Google Gemini Daily Brief](https://gemini.google/us/overview/daily-brief/?hl=en)
- [DailyStack](https://dailystack.ai/)
- [Nudget](https://nudget.app/)
- [Shortwave](https://www.shortwave.com/)
- [one sec](https://one-sec.app/)

### Gmail実装・審査要件

- Google, [OAuth restricted scope verification](https://developers.google.com/identity/protocols/oauth2/production-readiness/restricted-scope-verification)
- Google, [Cloud application security assessment](https://support.google.com/cloud/answer/13465431?hl=en)
- Google, [Gmail push notifications](https://developers.google.com/workspace/gmail/api/guides/push)
