# Tiimo Android 静的解析レポート

## まず読む

独自実装を始める場合は、最初に[機能とUX](03-features-and-ux.md)、次に[再実装ブループリント](05-reimplementation-blueprint.md)を読んでください。

このレポートは、Tiimo Android `1.1.4` のAPKを静的解析し、ADHD支援アプリを独立実装するための設計上の学びを整理したものです。ソースコードの復元・転載やTiimoのバックエンド利用を目的としません。

## 最重要の発見

Tiimoの中心的なドメイン分離は次の3段階です。

1. `Todo`: やる必要はあるが、実行時刻は未確定
2. `Activity`: 日時・時間帯・繰り返しが決まった予定
3. `Focus`: 今取り組むActivityを1件に絞った実行画面

この分離により、「思いついたことを忘れない」「いつ実行するか決める」「今は1つだけ見る」という、ADHDユーザーの異なる認知負荷を別々の画面で扱えます。

## 文書一覧

| 文書 | 内容 |
|---|---|
| [01 APK inventory](01-apk-inventory.md) | APK、Manifest、権限、SDK、解析条件 |
| [02 Client architecture](02-client-architecture.md) | 画面構造、状態管理、通知、課金、分析SDK |
| [03 Features and UX](03-features-and-ux.md) | ADHD支援機能、ユーザーフロー、設計原則 |
| [04 Network and backend](04-network-and-backend.md) | 観測API、認証、データモデル、責務境界 |
| [05 Reimplementation blueprint](05-reimplementation-blueprint.md) | 独自アプリの決定済みアーキテクチャと開発順序 |
| [06 Security, privacy, limitations](06-security-privacy-and-limitations.md) | クリーンルーム方針、個人情報、解析限界 |
| [Evidence](evidence/observations.md) | コマンド、ハッシュ、根拠となる観測一覧 |

## 確度の表記

- **確認**: APK、Manifest、DEX、Hermesバイトコードから直接確認
- **推定**: 複数の確認事項を組み合わせた合理的な推定
- **提案**: 独自アプリ向けの設計判断。Tiimoの実装事実ではない

## 調査範囲

- ADBで取得したbase APKと4つのsplit APK
- Hermes Bytecode v96の逆アセンブル・擬似コード化
- Manifest、DEX、Expo設定、ネイティブライブラリの静的確認
- ライブAPIへのアクセス、通信傍受、認証回避、動的計装は未実施
- SDKキー、DSN、トークン、ユーザーデータは収集・記載していない

## 結論

独自版のMVPでは、週次計画、Todo、Focus、ローカル通知、チェックリストだけを実装します。オンボーディング質問、AI分解、課金、外部カレンダー、マーケティングSDKは、利用価値が検証できるまで追加しません。
