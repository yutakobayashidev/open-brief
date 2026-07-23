# Evidence: Tiimo Android 1.1.4

## 解析日時・環境

- 日付: 2026-07-21
- 端末: Samsung SCG24
- Package: `com.tiimo.androidappreactnative`
- ADB: 36.0.1
- hermes-dec: 0.1.5, commit `66bb3449ace7ed48b400878da045e89e3a45bff2`
- Kirlif/HBC-Tool: commit `f6bb64cc6bfe6abe2084a025c249050e2be16921`
- 解析種別: local static analysis only

## APKハッシュ

```text
e216b58eca1fdf3f0dc32575066fc6d1e1f9ab2e4af18a3f8f89f8e9e05eeea0  base.apk
48f3e90aeea343bee3a8de4dfc8e38c7baa1500ccba99f61b26b5b8e791a0c8c  split_config.arm64_v8a.apk
0cff631a7c78aca421eb5e4bac91f72acf0da7770d690e832cd3f12b253e1074  split_config.en.apk
45f804da0e9a99eac97d58c1dace98fb90469a21554749b1453228cc8cf4d5ec  split_config.ja.apk
f13e7c272b42d6012dcfbb6c137fdbf3969b7333fa6bda5be462c460a255db86  split_config.xxhdpi.apk
```

端末上と取得後のSHA-256が5ファイルすべて一致しました。

## 主要コマンド

### APK取得

```bash
adb devices -l
adb shell pm path com.tiimo.androidappreactnative
adb pull <device-apk-path> <local-output>
sha256sum apks/com.tiimo.androidappreactnative/*.apk
```

### APK / Manifest / DEX

```bash
nix shell nixpkgs#unzip -c unzip -Z1 base.apk
nix shell nixpkgs#apktool nixpkgs#jadx -c \
  apktool d -f -s base.apk -o /tmp/tiimo-apktool
nix shell nixpkgs#apktool nixpkgs#jadx -c \
  jadx --no-res --no-debug-info --show-bad-code \
  -d /tmp/tiimo-jadx base.apk
```

### Hermes

```bash
nix shell nixpkgs#unzip -c unzip -p \
  base.apk assets/index.android.bundle \
  > /tmp/tiimo-index.android.bundle

PYTHONPATH=/tmp/openbrief-hermes-dec/src \
python3 -m hermes_dec.parsers.hbc_file_parser \
  /tmp/tiimo-index.android.bundle

PYTHONPATH=/tmp/openbrief-hermes-dec/src \
python3 -m hermes_dec.disassembly.hbc_disassembler \
  /tmp/tiimo-index.android.bundle /tmp/tiimo-disassembly.hasm

PYTHONPATH=/tmp/openbrief-hermes-dec/src \
python3 -m hermes_dec.decompilation.hbc_decompiler \
  /tmp/tiimo-index.android.bundle /tmp/tiimo-decompiled.js

PYTHONPATH=/tmp/openbrief-hbctool \
python3 -c 'import hbctool; hbctool.main()' \
  disasm /tmp/tiimo-index.android.bundle /tmp/tiimo-hbctool
```

### Native

```bash
readelf -h libappmodules.so
readelf -d libappmodules.so
nm -D --defined-only libappmodules.so
```

## Hermesヘッダー

| Field | Value |
|---|---:|
| Version | 96 |
| File length | 10,871,836 bytes |
| Function count | 48,945 |
| String count | 82,640 |
| Identifier count | 37,077 |
| RegExp count | 3,314 |
| Function source count | 702 |

`hermes-dec`と`HBC-Tool`の両方が、Source Hash
`59bd4d581fbc0be1540b6206065472fbaf0808d9`、HBC v96として正常に読み取りました。HBC-Toolは`instruction.hasm`、`metadata.json`、`string.json`を生成し、形式判定をクロスチェックしました。

## 直接確認したExpo Router files

```text
./(app)/(protected)/(tabs)/plan.tsx
./(app)/(protected)/(tabs)/focus.tsx
./(app)/(protected)/(tabs)/todos/index.tsx
./(app)/(protected)/(tabs)/todos/edit-lists.tsx
./(app)/(protected)/(tabs)/settings/index.tsx
./(app)/(protected)/(tabs)/settings/edit-profiles.tsx
./(app)/(protected)/(tabs)/settings/notifications.tsx
./(app)/(protected)/(tabs)/settings/sounds.tsx
./(app)/(protected)/(tabs)/settings/haptic-feedback.tsx
./(app)/(public)/sign-in.tsx
./(app)/(public)/onboarding/questions.tsx
./(app)/(public)/onboarding/routines.tsx
./(app)/(public)/onboarding/notifications.tsx
./(app)/(public)/onboarding/paywall.tsx
```

## 直接確認した製品component / function

```text
PlanScreen
TodosScreen
FocusScreen
FocusActivityCard
ActivityFormHeader / ActivityFormFooter
TodoForm / TodoFormHeader / TodoFormFooter
WeekCarousel
SuggestBreakdownButton
SelectTodoListModal
RoutinesScreen
QuestionsScreen
filterActivitiesForFocus
createActivityFromFormValues
prepareActivityFromTodo
```

## 直接確認したfeature flags

```text
feature_add_new_checklists
feature_add_new_profiles
feature_autobreakdown_checklist_free_tries
feature_custom_repetition
feature_edit_checklists
feature_focus_page
feature_multiple_todo_lists
feature_notifications
feature_search
feature_select_color
```

## 擬似コード上の主要位置

行番号は`hermes-dec 0.1.5`による今回の出力に対するものです。

| 内容 | `/tmp/tiimo-decompiled.js` |
|---|---:|
| Focus対象filter | 約806,900 |
| Activity API client | 約830,700 |
| Auth client / token refresh | 約844,900 |
| AI checklist | 約1,288,300 |
| Activity actions | 約1,367,500 |
| TodoList API | 約1,369,500 |
| Focus screen | 約1,434,400 |
| Todo API | 約1,433,400 |
| Profile API | 約1,483,800 |
| Plan screen | 約1,475,600 |
| Todo screen | 約1,499,700 |
| Onboarding questions/routines | 約1,552,900 |

## 確認した外部service種別

- Sentry
- Mixpanel
- Braze
- AppsFlyer
- RevenueCat
- Google Sign-In
- Expo Notifications

APK内にはこれらSDKの公開設定識別子が含まれますが、このリポジトリには値を転載していません。

## 再現上の注意

- `/tmp`の逆アセンブル・擬似コードは調査用で、リポジトリへ保存していません
- `hermes-dec`出力は有効な元JavaScriptではありません
- library codeと製品codeが同じbundleに含まれます
- 文字列の存在だけではruntime利用を証明できないため、複数のfunction/API/UI証拠を照合しました
- live APIへの接続、通信傍受、動的hookは行っていません
