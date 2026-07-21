# 02. クライアント・アーキテクチャ

## 全体像

Tiimo Androidは、React Native + Expoを中心にしたクライアントです。サーバー状態はTanStack Query、端末状態はZustand、機密tokenはSecureStoreへ分離されています。

```text
Expo Router screens
    │
    ├── form/domain adapters ── Zod + React Hook Form
    ├── server state ────────── TanStack Query ── Axios API clients
    ├── device state ────────── Zustand ───────── AsyncStorage
    └── native capabilities ─── Notifications / Audio / Haptics / SecureStore
```

## 画面構造

Expo Routerのファイルルートから、次の画面構成を確認しました。

### Public

- Landing
- Sign in
- Onboarding index
- Questions
- Routines
- Sign up / Sign up with email
- Notification permission
- Testimonials
- Paywall
- Preparing / Building loader

### Protected

- `Plan`: 週次予定とActivity編集
- `Todos`: Todoリスト、分類、並べ替え
- `Focus`: 今取り組むActivity
- `Settings`: profile、通知、音、ハプティック、製品更新
- Profile selection
- Marketing opt-in
- Paywall / Welcome loader

認証前後をrouter layoutで分離し、protected routesへのアクセスをguardしています。

## 状態管理

### Zustand: 端末・UI状態

| Store | 永続化 | 内容 |
|---|---|---|
| `useAppStore` | `app-storage` | theme、locale、週開始、12/24時間、音、haptic、Todo表示設定 |
| `useAuthStore` | `auth-storage` | user、認証済みフラグ。tokenは含めない |
| `useProfileStore` | `profile-storage` | 選択中Profile ID・名前 |
| `useFocusStore` | なし | 現在のfocus Activities |
| `useOnboardingStore` | なし | 質問回答、選択routine、新規登録状態 |
| `useSubscriptionStore` | なし | premium判定、RevenueCat customer info |

**設計上の学び:** 永続化すべきユーザー設定と、画面を閉じれば捨てられる一時状態をstore単位で分けています。tokenをauth storeへ混ぜていません。

### TanStack Query: サーバー状態

確認できたquery keyの例:

- Activities: `['activities', profileId, { fromDate, toDate }]`
- Todo lists: `['todo-lists', profileId]`
- Question set: `['questionSet', questionSetId]`
- AI checklist: `['aiChecklist']`

Mutation成功後は、局所的にキャッシュを書き換えるかinvalidateして再取得します。Todoのドラッグ中はローカル順序を先に更新し、API失敗時にサーバー状態へ戻します。

## フォームとドメイン変換

フォームはZod + React Hook Formを使い、UI用の値をそのままAPIへ送らず、変換関数を挟んでいます。

例:

- date/time入力をAPI日時へ変換
- `duration`の表示単位とAPI秒数を変換
- checklist itemsを作成・更新payloadへ変換
- recurring Activityの更新・削除範囲を明示
- TodoからActivityを組み立てる

独自版でも、画面componentから直接HTTP payloadを作らず、`form -> domain command -> API DTO`の順に変換します。

## UI技術

- Reanimated + Worklets + Gesture Handler
- Unistyles / React Native Paper
- SVG、Lottie、PagerView
- Keyboard Controller
- date/time picker
- i18nと複数locale

Focus画面の円形progress、Todoのdrag and drop、週carouselなど、操作量の多い部分にnative animation/workletを使っています。

## 通知

通知は2系統です。

1. Activityに紐づくローカル通知
2. Braze/Firebaseによるremote push・in-app message

ローカル側には次の操作があります。

- Activity通知をschedule/cancel
- 全Activity通知をcancel
- exact alarm permission確認・要求
- Android notification channel設定
- notification responseから対象画面へ遷移

**提案:** 独自版MVPはローカル通知だけで開始します。予定同期に成功した時点で次の通知を再構築すれば、サーバーpush基盤なしでも主要価値を提供できます。

## 認証と安全な保存

確認した認証方式はemail/password、Google Sign-In、access/refresh tokenです。API interceptorがBearer tokenと`Accept-Language`を付け、401時に1回だけrefreshします。失敗時は認証状態を破棄します。

独自版ではpassword grantを模倣せず、Authorization Code + PKCEまたはpasskeyを使います。token保存はSecureStore、ユーザー表示情報だけを通常storeへ置きます。

## 課金

RevenueCatを通じて次を扱っています。

- product/offerings取得
- 購入・復元
- paywall / customer center
- active entitlementによるpremium判定

機能flagには新規profile、checklist編集、通知、AI分解の無料回数、複数Todoリスト、カスタム繰り返し、Focus、検索、色選択などがあります。

**提案:** MVPではpremium code path自体を作りません。継続利用が確認できた後、AIコストや複数profileなど、明確に運用費・追加価値がある機能だけを課金対象にします。

## Analytics facade

確認した外部サービス:

- Sentry: crash、performance、API breadcrumb
- Mixpanel: page view、機能操作、作成・完了イベント
- Braze: push token、user attribute、engagement event
- AppsFlyer: attribution、onboarding complete
- RevenueCat: subscription identity

認証時には各SDKへuser IDを同期しています。イベントはMixpanel/Brazeへ直接散らさず、薄いanalytics facadeを通します。

独自版では次のinterfaceだけをdomainから呼びます。

```ts
interface ProductAnalytics {
  track(event: ProductEvent): void
  identify(userId: string): void
  reset(): void
}
```

MVP実装はno-opまたは自前の最小event tableとし、複数SDKは導入しません。

## 独自版への採用判断

| 観測技術 | 独自版MVP |
|---|---|
| Expo Router | 採用 |
| TanStack Query | 採用 |
| Zustand | 小さなUI設定だけ採用 |
| Zod + React Hook Form | 採用 |
| Reanimated / Gesture Handler | Focus progressと並べ替え時だけ採用 |
| RevenueCat | 後回し |
| Braze / AppsFlyer / Mixpanel | 不採用 |
| Sentry | beta以降に採用 |
