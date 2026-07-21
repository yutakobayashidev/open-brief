# 06. セキュリティ・プライバシー・解析限界

## クリーンルーム方針

独自版は、確認した機能境界と一般的な設計パターンだけを参考にします。

### 利用する情報

- Activity / Todo / Focusという抽象的な役割分離
- client/server責務、状態管理、通知の一般パターン
- 公開Manifestから分かるplatform capability
- 独自に設計し直したschema、API、UX原則

### 利用しない情報

- 復元されたTiimoコードの転載・移植
- Tiimoの文言、イラスト、icon、色、font、animationの複製
- Tiimoの商標、画面構成のピクセル単位の模倣
- Tiimo APIへの接続、認証tokenの取得、traffic replay
- SDK key、DSN、project ID、署名情報の転載
- 課金、license、PairIP、認証の回避

本レポート中のendpointは観測根拠を説明する目的であり、独自クライアントから呼び出しません。

## 静的解析の限界

- Hermes擬似コードは元JavaScriptではなく、変数名・型・制御構造が一部失われています
- minificationとlibrary codeが混在し、未使用機能の文字列も含まれる可能性があります
- endpoint文字列が存在しても、現在のproduction flowで必ず使われるとは限りません
- backend DB schema、認可rule、transaction、sync conflictはAPKだけでは確定できません
- feature flagにより、ユーザー・地域・課金状態で挙動が変わる可能性があります
- Manifest権限は依存SDKが宣言しただけで、アプリが実際に要求するとは限りません

したがって、観測したfieldやendpointをそのままproduction仕様とみなしません。

## 独自版の脅威モデル

保護対象:

- Todo、予定、routine、完了履歴
- ADHDや生活習慣を推測できる入力
- 通知内容
- access/refresh token
- AIへ送るtitle/notes

主な脅威:

- 別user/profileへのIDOR
- notification lock-screenからの内容漏えい
- analyticsへの過剰な本文送信
- backupからのtoken漏えい
- offline retryによる二重作成
- account削除後の残存データ

## セキュリティ要件

### Authorization

- path/queryのProfile IDだけを信用しない
- token subjectからmembershipをserver側で解決
- 全read/writeへ同じauthorization middlewareを適用
- sortable IDを使っても推測困難性を認可の代わりにしない

### Token

- Authorization Code + PKCE
- 短命access token、rotation付きrefresh token
- mobileではOS secure storageだけに保存
- AsyncStorage、SQLite、log、crash reportへtokenを出さない
- logoutとaccount削除時にserver sessionを失効

### Input / API

- shared Zod contractとserver-side validation
- request body、query、pathのsize上限
- mutationのidempotency key
- AI、login、password resetのrate limit
- stack traceや内部IDをclientへ返さない

### Mobile

- SecureStoreをcloud backup/device transferから除外
- exported componentを最小化
- FileProviderは非公開 + 一時URI permission
- exact alarm、overlay、録音権限は必要になるまで宣言しない
- production logにTodo本文やemailを出さない

## プライバシー要件

### Data minimization

- 必須profile fieldはtimezoneとlocaleだけ
- diagnosis、服薬、医療情報を要求しない
- analytics eventにtitle、notes、checklist本文を含めない
- AI利用は明示操作に限定

### Notification privacy

設定で次を選べるようにします。

- full titleを表示
- 「予定の時間です」だけ表示
- lock-screenでは非表示

### Retention / Export / Delete

- 完了履歴の保持期間を設定可能にする
- JSON exportを提供
- account削除はactive sessionを即失効
- DB、object storage、analytics identifierの削除期限を文書化
- backupからの最終消去期限もprivacy policyに記載

## AI利用時の追加要件

- title/notesを送る前に説明と同意
- providerの学習利用を無効化できる契約/APIを選ぶ
- prompt/outputへuser IDを含めない
- outputを命令ではなく編集可能な提案として表示
- health/safety判断をAIへ委ねない
- 失敗時は通常入力へ戻り、主要機能を止めない

## Accessibility

- 色だけで状態を表現しない
- dynamic type、screen reader label、44pt以上のtap target
- animation、sound、hapticを個別に無効化
- countdownを数字とprogressの両方で表示
- 「遅れ」「失敗」を責める文言にしない
- Focus中にmarketing/paywallを表示しない

## リリース前チェック

- [ ] 他user/profileアクセスのintegration test
- [ ] tokenがbackup/log/crash reportへ含まれない
- [ ] notification privacy設定がlock-screenで機能する
- [ ] account export/deleteが実機で完了する
- [ ] offline retryで重複Itemが作られない
- [ ] sound/haptic/animation offが全画面で尊重される
- [ ] privacy policyが実際のSDK・retentionと一致する
- [ ] Tiimo固有asset、文言、identifier、endpointを含まない

## 法的確認

本章は法的助言ではありません。公開前に、対象地域の著作権、利用規約、プライバシー、消費者保護、アクセシビリティ、医療表現に関する要件を専門家と確認してください。製品は診断・治療を提供するものではなく、日常の計画と実行を支援するツールとして表現します。
