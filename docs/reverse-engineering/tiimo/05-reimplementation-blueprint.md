# 05. 独自実装ブループリント

## ゴール

「思いついたTodoを、今日の予定へ移し、今やる1件として実行できる」Android/iOSアプリを構築します。MVPは個人利用、単一profile、オンライン同期あり、ローカル通知対応です。

## 技術選定

| 層 | 採用技術 | 理由 |
|---|---|---|
| Mobile | Expo + React Native + TypeScript | Android/iOS共通、通知・haptic・secure storage |
| Routing | Expo Router | public/protected/tabをファイル構造で分離 |
| Server cache | TanStack Query | API cache、mutation、optimistic UI |
| UI state | Zustand | theme、選択日、Focus一時状態だけを小さく保持 |
| Forms | React Hook Form + Zod | 入力とAPI DTOの境界を明確化 |
| Local storage | Expo SQLite | Inbox、plan cache、notification再構築 |
| API | Fastify + TypeScript | 型を共有しやすいmodular monolith |
| Database | PostgreSQL + Drizzle | relation、migration、日時query |
| Auth | OIDC Authorization Code + PKCE | password/token実装を自作しない |
| Push | Expo Push API | remote pushが必要になった段階で追加 |

Redis、microservice、event broker、GraphQL、課金SDK、AI SDKはMVPに導入しません。

## リポジトリ構成

```text
apps/
├── mobile/          Expo Router app
└── api/             Fastify modular monolith
packages/
├── contracts/       Zod request/response schemas
├── domain/          Pure domain functions and types
└── ui/              Shared design primitives
```

`domain`はReact、Expo、DB、HTTPをimportしません。`contracts`はmobile/apiの両方から参照します。

## ドメインモデル

### User / Profile

MVPは1 user = 1 profileですが、DBにはProfileを残します。将来の家族・支援者共有をmigrationなしで追加できます。

```text
users
- id, auth_subject, created_at

profiles
- id, user_id, display_name, timezone, locale
- week_starts_on, time_format
- sound_enabled, haptics_enabled, notifications_enabled
```

### Item

TodoとActivityを1つに統合します。

```text
items
- id, profile_id, list_id
- kind: todo | scheduled
- title, notes
- estimated_minutes
- time_bucket: anytime | morning | afternoon | evening | exact
- scheduled_start, scheduled_end
- recurrence_rule nullable
- color_token, icon_token
- sort_order
- created_at, updated_at, archived_at
```

Invariant:

- `kind=todo`なら`scheduled_start`はnull
- `time_bucket=exact`なら`scheduled_start`必須
- `estimated_minutes`は1〜1440
- user入力の色はdesign tokenへ正規化

### Checklist / Occurrence

```text
checklist_items
- id, item_id, title, sort_order

item_occurrences
- id, item_id, local_date
- scheduled_start, scheduled_end
- state: pending | active | paused | completed | skipped
- completed_at
```

繰り返しtemplateを編集しても、過去の完了履歴を壊さないためOccurrenceを分離します。

### Focus

```text
focus_sessions
- id, profile_id, occurrence_id nullable
- item_id
- started_at, paused_at, completed_at
- accumulated_seconds
```

countdownは端末で計算します。サーバーには毎秒送らず、start/pause/resume/completeだけを送ります。

## API

### Read

```text
GET /v1/bootstrap
GET /v1/plan?from=YYYY-MM-DD&to=YYYY-MM-DD
GET /v1/items?kind=todo&listId=
GET /v1/lists
GET /v1/focus-sessions/active
```

`bootstrap`はprofile、preferences、lists、今日のplanをまとめ、初回起動時のwaterfallを避けます。

### Mutation

```text
POST   /v1/items
PATCH  /v1/items/{id}
DELETE /v1/items/{id}
POST   /v1/items/{id}/schedule
POST   /v1/items/{id}/complete
PUT    /v1/items/reorder

POST   /v1/lists
PATCH  /v1/lists/{id}
DELETE /v1/lists/{id}

POST   /v1/focus-sessions
PATCH  /v1/focus-sessions/{id}
```

Mutation header:

```text
Authorization: Bearer <access token>
Idempotency-Key: <UUID>
```

すべてのresponseはserverのcanonical entityと`updatedAt`を返します。clientは成功responseでcacheを置き換えます。

## Mobile画面

### Inbox tab

- titleだけで即時保存
- 後からnotes、所要時間、checklistを追加
- 「今日へ移す」でtime bucketまたは時刻を選ぶ

### Plan tab

- 今日を初期表示
- 朝 / 昼 / 夜 / いつでも / 時刻指定のsection
- 7日stripで日付移動
- item tapで編集、checkboxで完了

### Focus tab

- active itemを1件表示
- remaining time、pause/resume、complete
- checklist
- 次の候補を見る操作は明示buttonにする

### Settings

- 通知、音、haptic
- theme、locale、週開始、12/24時間
- data export、account deletion

## 同期とオフライン

MVPの同期は「server authoritative + local cache」です。

1. Read結果をSQLiteへ保存
2. MutationをSQLiteへ即時反映
3. onlineならAPI送信
4. 失敗時はpending mutationとして保持
5. 再送時は同じidempotency keyを使う
6. 409時はserver entityを取得し、ユーザーへ再適用を提示

自動field mergeやCRDTは実装しません。同一項目を複数端末で同時編集した稀なケースは、server最新版を表示して再編集します。

## 通知

### MVP

- 端末ローカル通知のみ
- exact alarmを必須にしない
- scheduled itemの開始前に1回通知
- item更新・削除時にnotificationを再構築
- 端末再起動後、次の7日分を再schedule

### 後から追加

- remote pushは共有profile、サーバー生成予定、engagement通知が必要になった時だけ追加
- marketing pushはproduct notificationと別consentにする

## AI分解

AIはPhase 4まで実装しません。導入時のcontract:

```text
POST /v1/assist/checklist-suggestions
{ title, notes, maxItems: 5 }

-> { suggestions: [{ title }], modelVersion }
```

- ユーザーが明示buttonを押した時だけ送信
- suggestionは保存前に選択・編集可能
- health情報を送らない
- prompt/outputをanalyticsへ送らない
- timeout時も通常の手入力を妨げない

## 実装フェーズ

### Phase 1: Local vertical slice（3〜5日）

- Expo Router tabs
- SQLite schema
- Todo作成
- 今日へschedule
- Focus start/complete
- APIなし、端末内で完結

完了条件: 実機でTodo作成からFocus完了まで通せる。

### Phase 2: Backend sync（5〜8日）

- OIDC login
- Fastify + PostgreSQL
- Item/List API
- TanStack Query cache
- idempotent mutation retry

完了条件: 再インストール後にloginしてデータが復元される。

### Phase 3: ADHD UX（4〜6日）

- time bucket
- checklist
- local notification
- sound/haptic設定
- gentle rescheduling

完了条件: 10秒以内のcapture、3操作以内のschedule、2操作以内のFocus開始。

### Phase 4: 検証後の機能

- recurring item
- 複数list/profile
- AI checklist
- external calendar
- subscription

追加条件: 実ユーザーの反復利用か、具体的な運用コストの回収根拠があること。

## テスト戦略

### Domain unit tests

- Todoを各time bucketへschedule
- exact timeのvalidation
- recurrenceからOccurrence生成
- complete/pause/resume state transition
- DSTをまたぐ日付処理

### API integration tests

- 他ProfileのIDでread/writeできない
- 同じidempotency keyで二重作成されない
- reorderが重複/欠落IDを拒否
- account削除で関連データが削除/匿名化される

### Mobile E2E

- capture -> schedule -> focus -> complete
- offline作成 -> reconnect -> sync
- notification tap -> 対象item
- app再起動後もactive Focusを復元
- sound/haptic offを尊重

## 最初の1週間で作らないもの

- custom recurrence builder
- profile共有
- premium feature lock
- analytics SDKの多重導入
- AI自動分解
- external calendar OAuth
- drag可能な円形progress

最初の成功指標はダウンロード数ではなく、「作成されたTodoのうち、scheduleされFocusで完了した割合」です。
