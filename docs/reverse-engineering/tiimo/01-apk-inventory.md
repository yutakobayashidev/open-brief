# 01. APK・Android構成

## 結論

製品ロジックの中心はAndroidネイティブコードではなく、base APK内のReact Native/Hermes bundleです。Ghidraで`.so`を追うより、HermesとExpo Routerの解析を優先する方が効率的です。

## 対象バージョン

| 項目 | 確認値 |
|---|---|
| アプリ | Tiimo 1.1.4 |
| Package | `com.tiimo.androidappreactnative` |
| versionCode | 34 |
| Android min / target / compile SDK | 24 / 36 / 36 |
| Expo SDK | 55 |
| React Native | 0.83.6 |
| JavaScript engine | Hermes Bytecode v96 |
| React Native architecture | New Architecture、release level `stable` |
| Android Gradle Plugin | 8.12.0 |

Expo設定はportrait固定、light/dark自動切り替え、typed routes、React Compilerを有効化しています。Expo UpdatesはManifest上で無効であり、今回のJavaScript bundleはAPKに同梱されています。

## APKセット

Google Playの分割APKで、単体の`base.apk`だけでは完全な再インストールになりません。

| ファイル | 用途 | サイズ |
|---|---|---:|
| `base.apk` | DEX、resources、Hermes bundle、共通assets | 90,549,417 bytes |
| `split_config.arm64_v8a.apk` | ARM64ネイティブライブラリ | 23,980,642 bytes |
| `split_config.en.apk` | 英語resources | 111,001 bytes |
| `split_config.ja.apk` | 日本語resources | 53,657 bytes |
| `split_config.xxhdpi.apk` | 画面密度別resources | 128,663 bytes |

base APKには8個のDEXと、10,871,836 bytesの`assets/index.android.bundle`があります。Hermesヘッダーには48,945関数、82,640文字列が記録されています。

ハッシュは[Evidence](evidence/observations.md#apkハッシュ)に記載しています。

## Androidシェル

**確認:** `MainActivity`はSplash Screenを登録してReact Nativeを起動する薄いシェルです。`MainApplication`はExpoのhost factoryから同梱bundleをロードします。

Android XMLで独自画面を構築している構成ではありません。Android側の主な責務は次です。

- React Native / Expoの起動
- 通知、正確なアラーム、端末再起動後の復元
- foreground音声再生・録音
- Google Sign-In、課金、SecureStore
- Push、障害監視、分析SDKのnative bridge

## ネイティブライブラリ

ARM64 splitには30個の`.so`があります。主なものは以下です。

| ライブラリ | 役割 |
|---|---|
| `libhermesvm.so` | Hermes VM |
| `libreactnative.so` | React Native runtime |
| `libappmodules.so` | Fabric/Codegenで集約されたRN modules |
| `libreanimated.so`, `libworklets.so` | UI-thread animation/worklet |
| `libexpo-modules-core.so` | Expo native modules |
| `libsentry.so` | native crash reporting |
| `libNitroModules.so` | Nitro Modules bridge |

`libappmodules.so`の公開シンボルはScreens、SVG、Keyboard Controller、Braze、SentryなどのRN componentが中心です。製品のActivity/Todo/Focusロジックを読む主対象ではありません。

## Manifest権限

Manifestには依存SDK由来を含む43件の`uses-permission`があります。

### 製品機能に直結するもの

- 通知: `POST_NOTIFICATIONS`、exact alarm、再起動受信、Wake Lock、Vibrate
- 音声: 録音、音声設定、foreground media playback
- 通信: Internet、network/Wi-Fi state
- 認証・課金: Biometric/Fingerprint、Google Play Billing

### SDK・互換性由来

- 広告ID、AdServices attribution、install referrer
- Android 12以前限定の外部ストレージ
- 複数launcher向けbadge権限
- `SYSTEM_ALERT_WINDOW`

カメラ、位置情報、連絡先、端末カレンダーの権限は宣言されていません。外部カレンダー連携は、端末カレンダーを直接読むよりサーバー/OAuth経由と推定できます。

## Androidコンポーネント

| 種別 | 数 | 主な用途 |
|---|---:|---|
| Activity | 16 | RN host、Google認証、課金UI |
| Service | 8 | Push、通知、foreground音声 |
| Receiver | 7 | 通知操作、再起動、install referrer |
| Provider | 6 | Expo files、初期化、共有URI |

Exported componentは6件です。多くはpermissionで送信元を制限しています。`ClipboardFileProvider`は`exported=true`かつ明示permissionなしに見えるため、独自版では外部公開が必要かを設計段階で確認します。これは脆弱性認定ではなく、Manifestレビュー上の注意点です。

## 保存領域

**確認:** `allowBackup=true`ですが、Expo SecureStore領域はcloud backupとdevice transferの両方から除外されています。

独自版でも次の分離を採用します。

- UI設定・選択状態・非機密キャッシュ: AsyncStorageまたはSQLite
- access/refresh token: OS secure storage
- tokenや暗号鍵: バックアップ・端末移行対象外
- 添付ファイル: private app storage、必要なURIだけ一時grant

## 独自版で削るもの

最初からTiimoのManifestを再現しません。MVPではInternet、通知、Vibrateだけを基本とし、exact alarm、録音、foreground音声、広告ID、overlay、launcher badge、attributionは機能を実装する時点で追加します。
