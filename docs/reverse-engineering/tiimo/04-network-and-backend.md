# 04. ネットワーク・バックエンド境界

## 注意

この章はAPK内の静的文字列と擬似コードから確認したAPI境界です。実サーバーへリクエストしておらず、完全なOpenAPI仕様ではありません。独自版ではTiimoのhost、API、認証情報を利用せず、独立したcontractを定義します。

## 確認したサービス分割

| 境界 | 観測host | 役割 |
|---|---|---|
| Main API | `https://api.tiimoapp.com/api` | Profile、Activity、Todo、質問、AI checklist |
| Auth/Account | `https://auth.tiimoapp.com` | token、Google連携、userinfo、account操作 |
| AI helper | `https://ai.tiimoapp.com` | emoji/tag提案 |

## 観測API

### Activity

| 操作 | 観測method/path |
|---|---|
| 期間取得 | `GET /profiles/{profileId}/activities` |
| 作成 | `POST /profiles/{profileId}/activities` |
| 更新 | `PUT /profiles/{profileId}/activities/{activityId}` |
| 削除 | `DELETE /profiles/{profileId}/activities/{activityId}` |
| Todoへ変換 | `POST /profiles/{profileId}/activities/{activityId}/convert` |
| 完了・操作履歴 | `POST /profiles/{profileId}/activityactions` |
| checklist操作履歴 | `POST /profiles/{profileId}/checklistactions` |

期間取得には`fromDate` / `toDate`を使います。更新には`forDate`と`updateType`、削除には`forDate`と`deletionType`があり、繰り返し系列の「この回だけ / 以後 / 全体」のような範囲をサーバーへ伝える構成です。外部カレンダーID・event IDも削除payloadに現れます。

### Todo list / Todo

| 操作 | 観測method/path |
|---|---|
| List CRUD | `/profiles/{profileId}/todo-task-lists` |
| List削除 | `/profiles/{profileId}/todo-task-lists/{listId}` |
| List並べ替え | `PUT /profiles/{profileId}/todo-task-lists/reorder` |
| Todo CRUD | `/profiles/{profileId}/todo-tasks` |
| Todo削除 | `/profiles/{profileId}/todo-tasks/{taskId}` |
| Todo並べ替え | `PUT /profiles/{profileId}/todo-tasks/v2/reorder` |
| Activityへ変換 | `POST /profiles/{profileId}/todo-tasks/convert/{taskId}` |

### Profile / Onboarding / AI

| 操作 | 観測method/path |
|---|---|
| Profile CRUD | `/profiles`, `/profiles/{profileId}` |
| AI checklist | `GET /ai/checklist?title=...&description=...` |
| 質問セット | `GET /questionsets/{questionSetId}` |
| 回答送信 | `POST /answers/{profileId}` |
| emoji/tag提案 | `POST /v3/emojiUtfAndTagFromTitleAndTagNames` |

### Auth / Account

| 操作 | 観測method/path |
|---|---|
| ユーザー作成 | `POST /users/tiimo` |
| password sign-in/token | `POST /connect/token` |
| Google token交換 | `POST /connect/external/google` |
| user info | `/connect/userinfo` |
| email変更 | `POST /api/user/change-email` |
| account削除 | `DELETE /api/user` |
| subscription状態 | `GET /api/user/subscriptions` |
| password reset | `POST /api/users/{id}/reset-password` |

## 認証クライアント

**確認:** Axios clientは次を行います。

1. SecureStoreからaccess tokenを取得
2. `Authorization: Bearer ...`と`Accept-Language`を付与
3. 401時、同じrequestを1回だけretry可能にする
4. refresh tokenでaccess tokenを更新
5. refresh失敗時、tokenとローカル認証状態を破棄

独自版ではresource owner password grantを採用せず、mobile app向けAuthorization Code + PKCEを使います。API側はaccess tokenの`sub`とProfile membershipの両方を検証します。

## 観測データモデル

### Profile

ユーザーが扱うデータの所有・選択境界です。選択中Profile IDと名前を端末へ保存し、Activity/Todo APIの大半をProfile配下へscopeしています。

**推定:** 個人利用だけでなく、家族・支援者が別profileを扱う可能性を残す設計です。

### Activity

確認できた主なfield:

- `activityId`, `profileId`
- `title`, `description`
- `backgroundColor`, `iconType`, `iconId`, `iconUrl`
- `startDate`, `startTime`, `endDate`, `endTime`, `duration`
- `isAllDay`, time type
- `recurrenceType`, `repetition`
- `checklistItems`
- `state`, `type`
- `externalCalendarId`, `externalCalendarEventId`
- `taskEnrichmentId`

### TodoList

- `todoTaskListId`
- `title`
- `items`
- `sortOrder`
- `selectedGrouping`

### Todo

- `taskId`, `todoTaskListId`
- `title`, `notes`
- `duration`（APIでは秒）
- `backgroundColor`, icon fields
- `subTasks`
- `isChecked`
- `grouping`

### SubTask / Checklist item

- item ID
- title
- icon/color
- checked state
- index/order

### Action

Activity完了やchecklist checkを、entity本体の最終状態だけでなくactionとして送信しています。

**推定:** 日付別の繰り返しinstance、履歴、分析、複数端末同期を扱いやすくするevent記録です。

## Client / Server責務

| Client | Server |
|---|---|
| Form入力とZod検証 | user/profile ownership |
| 表示用日時・時間帯grouping | Activity/Todo永続化 |
| Focus countdownと進捗 | recurrence instance整合 |
| drag中の楽観的UI | reorder結果の正規化 |
| 音、haptic、ローカル通知 | action/check履歴 |
| query cacheと再取得 | 質問セット、AI提案 |
| locale/theme/端末設定 | token発行・更新 |

Focusのcountdownをサーバーでtickさせず、端末で計算し、開始・pause・completeの意味あるeventだけを送る分離は、通信量と故障点を減らします。

## 独自版のAPI contract

Tiimoのpathをコピーせず、MVPでは次の独立contractを使います。

| 操作 | 独自版endpoint |
|---|---|
| 期間plan取得 | `GET /v1/plan?from=&to=` |
| Item作成 | `POST /v1/items` |
| Item更新・削除 | `PATCH/DELETE /v1/items/{id}` |
| Todoをschedule | `POST /v1/items/{id}/schedule` |
| Item完了 | `POST /v1/items/{id}/complete` |
| List CRUD | `/v1/lists` |
| Focus開始 | `POST /v1/focus-sessions` |
| Focus pause/完了 | `PATCH /v1/focus-sessions/{id}` |

すべてのrequestでserverがuser/profile scopeをtokenから決定します。クライアントが送った`profileId`だけを信用しません。mutationにはUUIDのidempotency keyを付け、二重tap・retryによる重複作成を防ぎます。

## 独自版の単純化

観測モデルはActivityとTodoを別entityとして変換しています。独自版MVPでは、1つの`items` tableで状態を表します。

```text
Item
├── kind: todo | scheduled
├── title / notes / duration
├── scheduledStart / timeBucket
├── recurrenceRule
├── listId
└── checklistItems
```

TodoをActivityへ変換する操作は、別entityの複製ではなく`scheduledStart`または`timeBucket`の設定です。UIではTodo/Activityを明確に分けながら、DBと同期処理を簡潔に保ちます。

繰り返しだけは`item_occurrences`へ分け、日付ごとの完了をtemplateから独立して保存します。
