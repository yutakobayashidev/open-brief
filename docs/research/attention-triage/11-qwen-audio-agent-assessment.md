# 11. qwen-audio-agent調査とOpenBriefへの採用判断

## Status

- 調査日: 2026-07-29
- repository: [QwenAudio/qwen-audio-agent](https://github.com/QwenAudio/qwen-audio-agent)
- 調査commit: [`9074ca5c993fe35657828e54d98c86f99720e3a6`](https://github.com/QwenAudio/qwen-audio-agent/tree/9074ca5c993fe35657828e54d98c86f99720e3a6)
- package version: `0.9.1`
- license: Apache License 2.0
- 判断: **Activity Recall Timeline MVPへaudioを追加しない。常駐serviceと非同期処理の設計だけを参考にし、audioはGo後の任意voice bookmark / voice queryとして再評価する**

## 一文で言うと

`qwen-audio-agent`は活動記録器ではない。

> マイク音声をQwen Audio Realtimeへstreamし、会話を続けながらCodex等のbackend Agentへ非同期で仕事を委譲するvoice frontend / Gateway

画面、window event、system audioを受動記録して「いつ何をしていたか」を作る機能はない。OpenBriefと近いのは、常駐process、CLI、background work、adapter分離であり、primary use caseは異なる。

## Product boundary

[README](https://github.com/QwenAudio/qwen-audio-agent/blob/9074ca5c993fe35657828e54d98c86f99720e3a6/README_EN.md)と[architecture](https://github.com/QwenAudio/qwen-audio-agent/blob/9074ca5c993fe35657828e54d98c86f99720e3a6/docs/architecture.md)では、userから見える一つのassistantを内部で二層へ分ける。

```text
Microphone / text
        │
        ▼
Realtime frontend
  ├─ simple answer
  └─ spawn_thinking(objective)
             │
             ▼
       local Gateway queue
             │
             ▼
persistent Backend Agent Session
  └─ OpenCode / OpenClaw / Qoder / Hermes /
     CodeBuddy / Codex / generic ACP
```

Realtime frontendは会話、割込み、task status、permission replyを扱う。tool、file、code、長時間taskは一つのpersistent backend Agent Sessionへ渡す。WebUI、TUI、macOS desktopはreplaceable clientで、Gatewayだけがbackend processとtask stateを所有する。

## Audio pipeline

default pathは次である。

```text
microphone PCM
  └─ WebUIまたはTUI
       └─ local Gateway WebSocket
            └─ DashScope Qwen Audio Realtime WebSocket
                 ├─ ASR / turn detection
                 ├─ realtime response
                 └─ tool call
```

- inputは16 kHz mono PCM。
- WebUIはbrowser microphone、TUIはplatform別native adapterを使う。
- macOS TUIはecho cancellation付きfull duplex。
- Linux / WindowsはPortAudioのhalf duplexが既定。AECなしfull duplexはheadphoneを要求する。
- raw audio fileを保存する処理は見当たらず、audio chunkはmemory上でstreamされる。

provider URLはconfigで変えられるが、実装済みproviderはDashScope protocolだけである。[realtime provider](https://github.com/QwenAudio/qwen-audio-agent/blob/9074ca5c993fe35657828e54d98c86f99720e3a6/server/src/voice/realtime-provider.mjs)は`session.update`、`input_audio_buffer.append`、Qwen固有eventを使う。LM StudioのOpenAI互換Chat Completionsへ接続するadapterではない。

したがって「x870のLM Studioへ向ければlocal audioになる」とは判断しない。local realtime audioを使うには、対応modelだけでなくRealtime WebSocket protocol、ASR、turn detection、audio outputの別実装が必要である。

## Runtimeとmodule

root packageはNode.js `^22.22.2`、`^24.15.0`または`>=26.0.0`を要求するnpm workspaceである。

```text
cli/       command parse、起動、service管理
server/    Gateway、voice、task、agent、conversation
shared/    public protocol、runtime utility
web/       React WebUI
tui/       terminal UI、platform audio adapter
desktop/   macOS Electron app
```

調査commitでの単純なline countはsource約17,970行、test約9,076行だった。test fileは61件で、service unit生成、task lifecycle、ACP競合、signal、dependency directionまで広くfixture化されている。一方、実systemd restart、native audio、remote providerを含むend-to-end保証とは分けて読む。調査環境にNode.js / npmがなかったため、upstream testは実行せずsource inspectionだけを根拠にした。

特に[dependency boundary test](https://github.com/QwenAudio/qwen-audio-agent/blob/9074ca5c993fe35657828e54d98c86f99720e3a6/server/test/dependency-boundaries.test.mjs)は、documentだけでなくimport方向をtestしている。OpenBriefのmicro-crateにも同じ考え方を使える。

## Privacy boundary

[PRIVACY.md](https://github.com/QwenAudio/qwen-audio-agent/blob/9074ca5c993fe35657828e54d98c86f99720e3a6/PRIVACY.md)が明示するdata flowは次である。

- microphone audio、realtime transcript context、model response requestは既定でDashScopeへ送信
- delegated task、conversation context、resultは選択したbackend Agentへ送信
- backend Agentが使うmodel、tool、MCP、external serviceには別のprivacy policyが適用
- user profile、long-term memory、task state、configは`~/.config/qwaudio/`へ永続化
- uninstallはuser data directoryを自動削除しない
- Gatewayはloopback既定。remote公開時のTLS、auth、log、retentionはoperator責任

raw audioをlocal fileへ保存しないことと、audioがoff-deviceへ送られないことは別である。provider側のretentionをrepositoryは保証できない。

Activity Recallへ常時microphoneを加えると、本人以外の声、meeting、生活音、credentialの偶発送信が新しいriskになる。window timelineと5分screen captureでprimary hypothesisを検証できる段階では、このriskを追加しない。

## License

project codeは[Apache License 2.0](https://github.com/QwenAudio/qwen-audio-agent/blob/9074ca5c993fe35657828e54d98c86f99720e3a6/LICENSE)である。

codeをcopy / modifyして配布する場合、少なくとも次を管理する。

- Apache-2.0 license copy
- source attributionとcopyright / patent notice
- modified fileであることの表示
- repositoryの`NOTICE`が適用される場合のnotice保持
- [THIRD_PARTY_NOTICES](https://github.com/QwenAudio/qwen-audio-agent/blob/9074ca5c993fe35657828e54d98c86f99720e3a6/THIRD_PARTY_NOTICES.md)にある各dependencyのlicense

DashScope APIとQwen model serviceの利用条件はcode licenseと別である。OpenBriefはRust実装でproduct boundaryも異なるため、MVPではcodeをcopyせず、設計patternだけを参照する。

## OpenBriefが借りるもの

### 1. systemd user service lifecycle

[`gateway-service.mjs`](https://github.com/QwenAudio/qwen-audio-agent/blob/9074ca5c993fe35657828e54d98c86f99720e3a6/cli/src/gateway-service.mjs)はLinuxで次を実装している。

- user unitをXDG config配下へ生成
- `systemctl --user daemon-reload`
- `enable --now / start / stop / restart / disable`
- `Restart=on-failure`
- `KillMode=control-group`
- unit生成とcommand invocationをfakeでtest

OpenBriefの`enable / disable / status / watch`へ概念を採用する。Linux / niri MVPではlaunchdやWindows serviceを同時実装しない。

### 2. single process owner

Gatewayだけがbackend processとtask stateを所有し、UIを閉じてもbackground workは継続する。OpenBriefでもsystemdが起動する`openbrief watch`をcollectorとstoreの唯一のwriterにする。

state-changing CLI commandはmode fileを書き換えず、`${XDG_RUNTIME_DIR}/openbrief/control.sock`のUnix socketでcollectorへ送る。HTTP / WebSocket Gatewayは作らない。

### 3. bounded lane

[`task-scheduler.mjs`](https://github.com/QwenAudio/qwen-audio-agent/blob/9074ca5c993fe35657828e54d98c86f99720e3a6/server/src/task/task-scheduler.mjs)はglobal、owner、lane単位のconcurrency limitを持つ。

OpenBriefは一つだけ借りる。

```text
capture / VLM lane = 1
```

次の5分tickが来た時に前のrequestが実行中なら、新しい画像を作らず`model_busy_local` gapを残す。raw imageをmemory queueまたはdisk queueへ積まない。

### 4. atomic stateとcorruption quarantine

[`task-store.mjs`](https://github.com/QwenAudio/qwen-audio-agent/blob/9074ca5c993fe35657828e54d98c86f99720e3a6/server/src/task/task-store.mjs)はmode `0600`のtemporary fileからatomic renameし、壊れたJSONを上書きせずquarantineする。

OpenBriefでもconfig / small stateはatomic replaceし、corrupt fileを破壊的に修復しない。activity event本体はappend-only storeとretention testで扱う。

### 5. dependency direction test

OpenBriefはcrate graphを次へ固定し、CIで循環または逆依存を拒否する。

```text
cli → app
      ├─ core
      ├─ source
      ├─ capture
      ├─ model
      └─ store
```

## 借りないもの

| qwen-audio-agent | OpenBriefでの判断 |
|---|---|
| full-duplex realtime voice | MVP非採用 |
| ambient microphone / conversation transcript | MVP非採用 |
| DashScope dependency | 非採用 |
| persistent backend Agent Session | 非採用 |
| ACP / MCP delegation | 非採用 |
| multi-owner task scheduler | 非採用 |
| HTTP / WebSocket Gateway | Unix socketで十分 |
| WebUI、TUI、Electron orb | CLI-firstのため非採用 |
| user profileとlong-term memory | Activity Timelineには不要 |
| notification claim / spoken delivery retry | voice outputを持たないため不要 |

## 将来のaudio候補

audioを採用する場合も、観測sourceではなく任意のannotation / query interfaceから始める。

### A. Voice bookmark

```console
openbrief voice-note
```

push-to-talk中の一言だけをtranscribeし、現在時刻のActivitySliceへ`user_asserted` bookmarkとして付ける。

```text
14:03  user note: LM Studioのstructured outputを調べ始めた
```

- 完全optional
- goal入力の代替ではなく、本人が必要な時だけ使う
- ambient listeningなし
- raw audio非永続
- transcriptはmodel observationと区別して`user_asserted`

### B. Voice query

```text
「今日の14時ごろ何してた？」
    └─ openbrief around 14:00 --json
```

voice frontendはread-onlyの`today / around`結果だけを話す。screen image、raw event、store全体をvoice providerへ渡さない。

qwen-audio-agent自体を組み込むより、OpenBrief CLIまたは将来のread-only toolを外部voice agentから呼ぶ方が境界が小さい。

## 導入gate

audio検討は次をすべて満たした後に行う。

1. Activity Recall R0 / R1の3日experimentがGo。
2. `today / around`だけでは想起できない区間が複数回ある。
3. 本人がtypingよりvoice bookmarkまたはvoice queryを使いたいと確認する。
4. push-to-talkとremote送信を毎回認識できるUIを作れる。
5. provider、retention、bystander audioのriskをscreen captureと別のconsentで扱える。

最初のaudio spikeは常時録音ではない。

> 一言のvoice bookmarkが、手入力なしtimelineで残る`不明`を、privacy costに見合うだけ減らせるかを検証する。
