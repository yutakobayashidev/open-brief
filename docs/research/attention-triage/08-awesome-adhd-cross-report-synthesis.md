# 08. awesome-adhd横断レポート

## Status

- 調査日: 2026-07-27
- 対象: `awesome-adhd`のconcept、entity、query、source policy
- 調査範囲:
  - concept: 41件
  - entity: 6件
  - query: 5件
  - comparison: 1件
  - raw paper note: 48件
  - raw article note: 66件。このうちX投稿由来は50件
- 位置づけ: 設計仮説の横断整理。ADHDへの治療効果またはOpenBrief固有の効果を示すものではない
- repository: `~/ghq/github.com/yutakobayashidev/awesome-adhd`

## 総合結論

`awesome-adhd`全体から最も一貫して言えることは次です。

> ADHD支援の中心は「集中力を強くする」ことではなく、記憶、時間、着手、終了、切り替えを本人の頭だけに保持させず、環境へ移し、中断しても戻れるようにすること。

問題を「集中できるか」だけでなく、次の状態遷移として捉えます。

```text
意図を持つ
    ↓
始める
    ↓
続ける
    ↓
終える
    ↓
中断をまたぐ
    ↓
元の文脈へ戻る
    ↓
あとで実行する
```

OpenBriefへの含意は、ADHDの人を自動管理する巨大な生産性アプリを作ることではありません。

> 本人の意図を守り、必要な認知作業だけを環境へ移し、中断しても恥をかかず次の有効行動へ戻れる、小さく調整可能な支援層を作る。

## 1. タスクより遷移を設計する

複数のconceptを横断すると、困難はタスク内容より境界に集中しています。

- やることは分かるが始められない
- 始めた後に終了できない
- 通知、会議、会話、tab移動の後に戻れない
- `あとでやる`を実行時点まで運べない
- 次の予定があるだけで、それまでの時間を使えない

したがって機能単位をTodoだけにせず、次の境界部品として設計します。

- 開始を小さくする
- 開始前に終了条件を置く
- 中断前に再開点を残す
- 終了後に次の一手を返す
- 次回確認の時刻または文脈を決める

この考え方はOpenBriefの`Protect → Signal → Explore / Focus → Capture → Return`を、単なる情報整理ではなく注意遷移protocolとして扱う方針と一致します。

### 参照したawesome-adhdページ

- `concepts/executive-function.md`
- `concepts/task-initiation.md`
- `concepts/task-resumption.md`
- `concepts/prospective-memory.md`
- `concepts/waiting-mode.md`

## 2. 認知を環境へ移す

Wikiで最も繰り返される原則は外部化です。

| 内部で保持していたもの | 外部化する先 |
|---|---|
| 記憶 | メモ、固定場所、checklist、状態表示 |
| 時間 | 視覚timer、完了時刻、次予定までの残り |
| 判断 | 前夜の準備、固定手順、選択肢削減 |
| 着手 | 予約時刻、他者の存在、最初の身体動作 |
| 終了 | alarm、blocker、退室、終了宣言 |
| 再開 | file、URL、cursor、次の一手、Return Anchor |
| 将来意図 | `いつ・どこで・何を`の文脈trigger |

設計目標は「気をつける」ことではなく、「忘れても事故になりにくく、思い出す作業をしなくても次の行動が見える」状態です。

ただし、外部記憶を増やしすぎると、保存先、検索、整理が新しい実行機能負荷になります。置き場所と表示先は少数に絞ります。

### 参照したawesome-adhdページ

- `concepts/external-memory.md`
- `concepts/environment-design.md`
- `concepts/forgetfulness-countermeasures.md`
- `concepts/working-memory.md`
- `concepts/time-management.md`

## 3. 有限性が注意と安心を作る

時間盲、過集中、waiting mode、FoMOは別々の問題に見えます。しかし、終端、残量、次回確認が曖昧なものが現在の注意全体を占有する点で共通しています。

有望な表示は次です。

- 今回扱う件数
- 推定所要時間
- 残り件数
- `今回はここまで`
- 次回確認時刻
- 終了後に戻る場所

通知や情報アクセスを全面遮断すれば必ず改善するとは限りません。見逃し不安が強い場合は、完全遮断が確認衝動を増やす可能性があります。

したがってOpenBriefでは、禁止より次を使います。

```text
今回確認する範囲
    +
確認済みの範囲
    +
明示的な終端
    +
次回確認の約束
    +
Return Anchor
```

### 参照したawesome-adhdページ

- `concepts/fear-of-missing-out.md`
- `concepts/digital-interruptions.md`
- `concepts/hyperfocus-control.md`
- `concepts/time-management.md`
- `queries/toymaker-openbrief-adhd-design-notes.md`

## 4. 刺激は減らすのではなく選別する

Wiki内には一見矛盾する知見があります。

- 静かで低刺激なUIがよい
- 進捗、好奇心、ゲーム性が着手を助ける場合がある

総合すると、刺激の総量ではなく方向が重要です。

- 通知、視覚noise、無関係な選択肢は減らす
- 次の一手、意味ある進捗、課題関連の好奇心は見えるようにする
- 製品への滞在を増やす報酬設計は避ける
- 本人の目的へ戻す刺激だけを使う

ゲーミフィケーションを一律禁止するのではなく、その刺激が本人の目的を助けるか、製品利用時間を増やすかで判断します。

### 参照したawesome-adhdページ

- `concepts/attention-control.md`
- `concepts/curiosity-reward-memory.md`
- `concepts/hyperfocus-control.md`
- `concepts/passive-memory-assistants-adhd.md`

## 5. 人の存在も外部足場になる

Body doublingやFocusmate型の価値は、相手に監督されることではありません。

- 開始時刻が決まる
- 最初にやることを宣言する
- 有限な作業枠ができる
- 最後に状態と次の一手を言葉にする

同期会議も常に悪いわけではありません。

| 状況 | 合いやすい様式 |
|---|---|
| 情報共有、読み返し、処理時間が必要 | 非同期文書 |
| 発言準備が必要 | 事前文書と短い同期 |
| 着手できない | 共同作業枠 |
| 前提の食い違い、感情的な修復 | 短い同期とその場での文書化 |
| 会議後に戻れない | Return Anchor |

問題は同期か非同期かではなく、処理時間、作業記憶、感覚負荷、切り替え負荷、社会的安全、説明責任との文脈適合です。

他者へ見せるのは作業名、時間枠、本人が選んだ状態に限定します。本文、金額、宛先、画面、注意推定は共有しません。

### 参照したawesome-adhdページ

- `concepts/body-doubling.md`
- `concepts/async-meetings-context-fit.md`
- `entities/focusmate.md`
- `queries/toymaker-neurodivergent-async-meetings-ai-2026.md`

## 6. パーソナライズは小さな自己実験で行う

同じ支援でも、人、時間帯、疲労、作業内容、不安、感覚条件によって逆効果になります。

Wikiが支持するのは、固定された万能な`ADHD mode`ではなく、小さな自己実験です。

1. 一度に一条件だけ変える
2. 行動結果を見る
3. 続かなかったことを人格失敗にしない
4. 合わなければ捨てる
5. 状況が変わったら再試行できる

評価対象も、一般的な集中力scoreではなく次の条件差にします。

- どの条件で読みやすいか
- どの条件で始めやすいか
- どの条件で戻りやすいか
- どの条件で確認衝動が弱いか
- 支援自体の修正負担はどれくらいか

自己報告だけでなく、実際の復帰、編集、実行、提出、返信などの行動も観測します。

### 参照したawesome-adhdページ

- `concepts/self-experimentation.md`
- `concepts/cognitive-personal-informatics.md`
- `concepts/digital-adhd-support.md`

## 7. AIはTaskmasterではなくCo-regulatorにする

AIに適する役割は次です。

- 長文の短文化
- taskの小さい手順への分解
- 次の一手候補
- 中断前の文脈再構成
- 会議前の論点整理
- 不明点、根拠、低信頼箇所の表示
- 本人が選ぶ通信状態の候補生成

AIに任せない役割は次です。

- `怠けている`、`脱線した`、`集中している`という断定
- ADHDまたは神経型の推定
- AI判断だけをtriggerにした通知、DND、status変更
- 重要度または人生目標の最終決定
- 行動logの家族、学校、職場への自動共有

AI出力では次の契約を採用候補にします。

- 答えと次の行動を先に置く
- 手順を小さい番号付きlistにする
- 現在地と完了したことを見えるようにする
- 最後に2分以内の次行動を1つだけ置く
- errorは原因、場所、修正方法を人格評価なしで示す

### 参照したawesome-adhdページ

- `entities/i-have-adhd.md`
- `concepts/assistive-technology.md`
- `concepts/speech-to-text-neurodiversity-support.md`
- `raw/papers/deshmukh-2025-neurodivergent-aware-productivity-ai.md`

## 8. 支援そのものが障害になりうる

外部化と自動化には反対方向のriskがあります。

- メモ先を増やすと探索先が増える
- 通知を増やすと無視、不安、通知疲れが増える
- 全記録すると検索と整理が新しい仕事になる
- dashboardは自己批判を増やす
- 自動分類は訂正作業を増やす
- 行動計測は監視へ転用できる
- Captureは新しい未処理backlogになりうる

よい支援は情報を増やすものではなく、認知上の未決定事項を減らすものとします。

### Guardrails

- 1画面1目的
- 最新1〜3件だけを表示する
- 未処理件数を人格評価または義務として表示しない
- 短い既定retention
- one-click pauseと期間指定削除
- 行動dataを本人側に閉じる
- 観測、AI推定、本人確認を分離する
- source of truthを複製しすぎない

## 製品群から見える市場の穴

既存製品は概ね次へ分断されています。

| 製品群 | 主に解くもの | 足りないもの |
|---|---|---|
| Screenpipe、Recall等 | 受動記憶 | ADHD-informedな復帰と低負荷UI |
| Granola、Otter等 | 会議記録 | 会議前の作業への復帰 |
| one sec、ScreenZen等 | 衝動前の摩擦 | 記憶と再開 |
| Tiimo等 | 時間定位、次予定、task分解 | 受動context |
| Focusmate | 着手、有限枠、終了確認 | 個人contextの保持 |
| Workona、Raycast等 | 作業文脈整理 | 注意摩擦と時間定位 |

調査queryが提示する市場の穴は次です。

> 受動記憶 × 注意摩擦 × 時間定位 × 低認知負荷UI

ただし、すべてを一つの巨大appへ統合するという意味ではありません。必要な部品だけをAttention Handoffとして接続します。

### 製品から借りる設計

- Screenpipe: event-driven capture、local store、検索API、権限境界
- Tiimo: appを開かなくても次行動が見えるambient UI
- Focusmate: 開始宣言、有限作業枠、終了確認
- Genio Notes: 同時処理を後処理へ分離し、本人が取捨選択するUI
- i-have-adhd: answer-firstな出力契約
- one sec: 罰ではなく本人の意図を再提示するfriction

各製品の公式説明は機能の根拠であり、ADHDへの独立した効果証拠ではありません。

### 参照したawesome-adhdページ

- `concepts/passive-memory-assistants-adhd.md`
- `queries/toymaker-passive-memory-adhd-design-2026.md`
- `entities/screenpipe.md`
- `entities/tiimo.md`
- `entities/focusmate.md`
- `entities/genio-notes.md`
- `entities/i-have-adhd.md`

## 主要な緊張

各方向を一律なruleにすると、別の利用者または場面で逆効果になります。

| 緊張 | 統合した判断 |
|---|---|
| 静かなUI vs 興味と報酬 | 無関係刺激を減らし、課題関連刺激を残す |
| 通知遮断 vs FoMO | 全遮断ではなく有限batchと次回確認 |
| 自動化 vs 自律性 | 観測、推定、本人確認を分離する |
| 非同期 vs 外部説明責任 | 情報共有は非同期、開始、共同判断、修復は短い同期 |
| 記録 vs 監視 | data所有と解釈を本人側に閉じる |
| 過集中の活用 vs 消耗 | 向け先と終了条件を同時に設計する |
| 標準化 vs 個人差 | 最小baselineと個人内比較を使う |
| gamification vs 依存 | 本人の目的を助ける刺激か、製品滞在を増やす刺激かで判断する |

## 反対仮説

総合仮説をそのまま製品仕様にせず、次を競合仮説として残します。

### H-alt-1: AIは不要

一行の次行動メモ、固定時刻、deep linkだけで効果の大半を得られる。

### H-alt-2: ADHD固有ではない

明確な次行動、通知整理、作業復帰、低認知負荷UIは一般的な良いHCIであり、ADHD特化の追加価値は小さい。

### H-alt-3: 外部化が逆効果になる

メモ、通知、履歴、AI候補を増やすほど、新しい管理対象とInboxが増える。

### H-alt-4: センシングが状態を悪化させる

行動記録は支援より、自己監視、不安、職場評価、maskingを強める。

### H-alt-5: 構造が反発を生む

強いblock、deadline、通知は一部の利用者を助けるが、別の利用者には回避とreactanceを生む。

## エビデンスの強さ

このWikiのsource policyは、研究・guideline、製品説明、当事者知を区別しています。

### 比較的強い

- NICE等の診断・管理guideline
- 薬物療法、CBT、一部digital therapeuticのsystematic reviewまたは比較研究

これらも対象年齢、期間、outcome、研究間の異質性に限界があります。

### 中程度または隣接研究

- 通知batching
- 妨害刺激
- prospective memory
- task resumption
- FoMO
- digital cognitive training

一部は一般集団、横断研究、自己報告で、OpenBriefまたはADHD特化UIを直接検証していません。

### 設計仮説として扱う

- body doubling
- 具体的なtask initiation tactic
- time management hack
- environment design
- self-experimentation
- Screenpipe、Tiimo、Focusmate等の製品mechanism

これらはX投稿、製品説明、実務記事を多く含み、ADHD症状への効果を証明するものではありません。

### Repository上の制約

- concept 41件のconfidenceはhigh 1件、medium 25件、low 15件
- entity 6件のconfidenceはhigh 1件、medium 5件
- query 5件はすべてmedium
- 一部raw paper noteは本文またはabstractを保持せず、DOI metadataだけ
- 一部deep research reportには内部引用IDが残り、現在のrepositoryだけで一次資料まで追跡できない主張がある
- 小児、青年、成人、診断群、自己申告症状群、自閉特性を含む研究が混在する
- `awesome-adhd`のライセンスは未決定で、再利用権は明示されていない

したがって、この調査から強く言えるのは「有望な設計方向」であり、「この方法または製品がADHDを改善する」という効果主張ではありません。

## OpenBriefへ持ち込む設計判断

### 持ち込む

- 注意状態より、本人が守りたい意図を中心にする
- Briefに範囲、終端、残件、次回確認、Return Anchorを持たせる
- 観測、AI推定、本人確認を分離する
- 支援presetを本人が選ぶ
- 一行メモ、deep link、固定確認時刻をbaselineにする
- 評価をapp滞在時間ではなく`substantive return`で行う
- `保留 / 確認待ち / 次回 / 今は戻らない`を使う
- 自動captureは価値検証後に段階的に追加する

### 持ち込まない

- ADHDまたは神経型の自動推定
- Captureの自動Todo化または自動Wiki化
- 行動dataの管理者向け分析
- productivity score、streak、ranking
- 全画面、音声、全key入力の既定取得
- AIによる重要度と注意状態の最終決定
- 蓄積量を成功指標にする設計

OpenBriefとLLM Wikiの境界も維持します。

```text
OpenBrief
  いま読む、守る、退避する、戻る
          ↓ 本人が残すと決めたものだけ
LLM Wiki
  長期的に知識化し、再利用する
```

蓄積するほど価値が上がるLLM Wikiの論理をOpenBriefへ持ち込むと、新しいInboxになります。

## 今後の研究質問

- Resume Packはdeep linkまたは一行メモより本当に優れるか
- `Observed / Inferred / User-confirmed`をどう表示すれば誤誘導を防げるか
- 中断前、会議直後、app復帰時のどこで提示するのがよいか
- 復帰支援に必要な最小dataは何か。screen、audioは追加価値を持つか
- ADHD群で一般利用者より大きな効果が出るか
- 不安、自閉特性、疲労、作業種別によって効果がどう変わるか
- 数週間後に通知無視、依存、管理負荷がどう変わるか
- body doubling、非同期brief、短い同期はどの条件で効くか
- 主観的な安心と、実際の返信、提出、復帰が一致するか
- 支援を停止した時にも効果が残るか

## 現在の判断

`awesome-adhd`全体は「万能なADHD生産性AI」を支持していません。

支持しているのは、次の条件を満たす支援です。

- 小さい
- 低摩擦
- 本人が調整できる
- 失敗を人格評価しない
- 必要な時だけ認知作業を引き取る
- 中断しても復旧可能
- 行動dataを本人側に閉じる
- 効果がなければ捨てられる

OpenBriefの有望な位置づけは、ADHD向けlife-logまたはtask managerではなく、**中断をまたぐAttention Handoff layer**です。
