# 01. 認知科学・HCIの研究基盤

## 結論

ClawBrifは「集中力を増やすアプリ」ではなく、好奇心を残しながら、遅延した意図と元作業への復帰経路を保護するシステムとして設計します。

研究から直接言えるのは、好奇心には学習上の価値があること、情報取得や通知の頻度を有限化すると負担が下がる場合があること、中断後の復帰には外部手掛かりが役立つことです。

一方、有限トピックブリーフ、Curiosity Capture、Return Anchorを組み合わせた効果は未検証です。ここがClawBrifの研究対象です。

## 対象となる問題

```text
返信・義務
├── 報酬が遅い
├── 開始点が曖昧
└── 実行まで意図を保持する必要がある

ニュース・SNS
├── 新奇性と不確実性がある
├── 情報取得自体に価値がある
└── 次の項目が無制限に供給される

競合すると
└── 元の意図が見えなくなり、探索後の復帰に失敗する
```

この連鎖全体をADHD成人で直接実証した研究はありません。各要素を別の研究が支え、統合した因果モデルは製品仮説です。

## エビデンス表

| 論点 | 主な結果 | Tier | 設計への含意 |
|---|---|---|---|
| 好奇心 | 好奇心は情報を得るための行動、報酬関連活動、記憶向上と関連した | E2 | 好奇心を除去せず、安全な探索方法へ変換する |
| Hyperfocus | ADHD特性と長時間の没入に自己報告上の関連がある | E3 | 診断語ではなく、切替困難を行動として測る |
| 遅延した意図 | ADHD成人の実験では、記憶保持より計画形成と切替で大きな差が出た | E2 | 「後で」を具体的な次の一手と時刻へ変換する |
| 中断と再開 | 中断後の復帰には元目標の文脈・視覚的位置などの手掛かりが役立つ | E2 | 探索前にReturn Anchorを保存する仮説へつなげる |
| 終了手掛かり | reading-history labelや「確認済み」が終了基準として使われた | E2/E3 | 残件数と明示的な終端を検証する |
| 通知バッチング | 予測可能な時刻への通知集約が、通常通知より注意や統制感を改善した | E2 | 収集は常時、提示は1日2回から始める |
| 通知なし | 通知を表示しない条件でFoMOや不安が高かった | E2 | 情報アクセスを禁止せず、override可能にする |
| 価値ベースの制御 | 本人が無価値と判断した利用だけを減らし、価値ある利用を維持できた | E2 | 総利用時間ではなく、後悔する利用を減らす |

E1は空欄です。ClawBrif固有の効果は、実装後のN-of-1から初めて埋まります。

## 好奇心は保護対象である

Kangらの実験では、好奇心の高さが情報を得るための資源支出、報酬関連領域の活動、後の記憶と関連しました。好奇心を単なる誘惑として扱うと、ClawBrifが守るべき学習と着想まで損ないます。

KobayashiとHsuは、情報の主観的価値と金銭的報酬に共通する神経表現を報告しました。ただし、これを「SNSは薬物と同じ」「ドーパミン依存」と言い換えることはできません。

設計上の結論は、情報価値を否定せず、無境界な供給形式だけを変更することです。

## Hyperfocusの扱い

成人Hyperfocus質問紙の研究では、ADHD症状との関連や、趣味・画面利用など複数場面での自己報告が示されています。ただし、初期研究は新規尺度と自己報告診断を多く含みます。

したがって、研究文書では次の観測可能な語を使います。

- 新奇情報による注意捕捉
- 探索からの離脱困難
- 意図したタスク切替の遅延
- 元作業への復帰失敗

ClawBrifはADHDの診断、症状判定、治療を行いません。

## 遅延した意図と計画

Fuermaierらは、ADHD成人45人と対照45人を比較し、複雑なprospective memory課題で、特に計画形成とタスク切替に大きな群差を報告しました。一方、計画の想起と自己開始の差は小さい、または有意でない部分がありました。

この結果から「ADHDの人は返信を忘れる」と一般化はできません。製品上は、返信候補をInboxへ置くだけでなく、次の物理的行動、実行時刻、実行文脈を本人が確定できるようにします。

## 有限性と提示タイミング

Baughanらは、43人が4週間使うカスタムTwitterクライアントへreading-history label、list、時間dialogなどを導入しました。「確認済み」を終了基準にしたという知見には質的発言が含まれ、日次集約データから因果方向を確定できない限界があります。これは有限トピックブリーフの直接証拠ではなく、明確な終端を検討する近接証拠です。

Fitzらの237人のランダム化field studyでは、通知を1日3回にまとめた条件で注意、生産性、気分、統制感が改善しました。通知を表示しない条件では不安やFoMOが高くなりましたが、アプリを開いて情報へアクセスすること自体は禁止されていません。

KushlevとDunnの124人の被験者内実験では、メール確認を1日3回に制限した週の方が、無制限の週より日々のストレスが低くなりました。

ClawBrifはこれらから「収集は常時、提示は予測可能な有限セッション」という仮説を導きます。

## 再開手掛かり

RatwaniとTraftonの研究では、元作業の視覚的位置が中断後の再開を導きました。MasicampoとBaumeisterは、未完了目標について具体的な計画を作ることで、別課題への認知的干渉が下がることを報告しました。

ClawBrifのReturn Anchorは次を保存します。

```text
戻る対象: 認証テストの修正
再開点: refresh tokenの失敗ケース
次の一手: 401を期待するtestを1件追加
戻る時刻: 13:00
```

この短いカード自体の効果は未検証です。既存研究から導いた、検証対象の設計仮説です。

## 直接は支持されていない主張

- 無限スクロールがADHD固有の依存を起こす
- HyperfocusがADHDの確立した中核症状である
- ニュースを義務の報酬にすれば実行率が上がる
- AIの重要度判定が人間の判断より正確である
- 情報閲覧時間が短いほど健康または生産的である

## 主要文献

- Kang et al. (2009), [Epistemic Curiosity Activates Reward Circuitry and Enhances Memory](https://doi.org/10.1111/j.1467-9280.2009.02402.x)
- Kobayashi & Hsu (2019), [Common neural code for reward and information value](https://doi.org/10.1073/pnas.1820145116)
- Hupfeld et al. (2019), [Living in the zone: hyperfocus in adult ADHD](https://doi.org/10.1007/s12402-018-0272-y)
- Hupfeld et al. (2024), [Validation of the dispositional adult hyperfocus questionnaire](https://doi.org/10.1038/s41598-024-70028-y)
- Forster et al. (2014), [Increased distraction by task-irrelevant stimuli in adults with ADHD](https://doi.org/10.1037/neu0000020)
- Fuermaier et al. (2013), [Complex Prospective Memory in Adults with ADHD](https://doi.org/10.1371/journal.pone.0058338)
- Jylkkä et al. (2023), [Everyday prospective memory in adult ADHD](https://doi.org/10.1038/s41598-023-36351-6)
- Ratwani & Trafton (2008), [Spatial memory guides task resumption](https://doi.org/10.1080/13506280802025791)
- Masicampo & Baumeister (2011), [Plan making can eliminate cognitive effects of unfulfilled goals](https://doi.org/10.1037/a0024192)
- Baughan et al. (2022), [How Design Influences Dissociation on Social Media](https://doi.org/10.1145/3491102.3501899)
- Fitz et al. (2019), [Batching smartphone notifications can improve well-being](https://doi.org/10.1016/j.chb.2019.07.016)
- Kushlev & Dunn (2015), [Checking email less frequently reduces stress](https://doi.org/10.1016/j.chb.2014.11.005)
- Hiniker et al. (2016), [MyTime: Designing and Evaluating an Intervention for Smartphone Non-Use](https://doi.org/10.1145/2858036.2858403)
