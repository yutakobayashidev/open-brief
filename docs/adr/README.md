# Architecture Decision Records

このディレクトリには、ClawBrifの実装を長期的に拘束する技術判断を記録します。

## 運用

- 1判断につき1ファイルを使う
- ファイル名は`NNNN-short-decision-name.md`とする
- 合意前は`Proposed`、採用後は`Accepted`とする
- 判断を変更するときは過去の記録を書き換えず、新しいADRから`Superseded`にする
- 実装や設定を変更するときは、関連ADRも同じ変更で更新する

## 一覧

| ADR | Status | Decision |
|---|---|---|
| [0001](0001-adopt-local-first-data-and-model-boundaries.md) | Accepted | local-firstなデータ境界と、ユーザーが選べるModel Gatewayを採用する |
| [0002](0002-adopt-attention-signals-and-slack-status-output.md) | Accepted | Signal段階とSlack Status Output Adapterを採用する |
