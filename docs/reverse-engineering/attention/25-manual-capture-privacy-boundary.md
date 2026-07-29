# 25. Manual captureとhistory readのprivacy境界

## 結論

Attentionの現在画面を取得するmanual RPCは、periodic recorderと同じprivacy policyを通らない。共通して確認できたgateはmacOSのScreen Recording permissionだけで、pause、app / window / domain exclusion、private browsing、notification exclusion、overlay recording gate、commit epochは共有しない。

`query image`はさらに別のhistory readである。保存済みframeをIDまたは時刻で取得するため、現在のcapture permissionや現在の除外設定を再評価する証拠はない。

OpenBriefでは、capture sourceごとにsecurity policyを複製しない。periodic、manual、Agent requestを一つの`PolicyGate`へ集約し、`live_screen_read`と`history_read`を別capabilityにする。

## Current screen capture

確認したentry:

| Address | Path |
|---|---|
| `FUN_1000981f0` | current screen capture |
| `FUN_10009afa4` | window-targeted capture |

current screen pathは`CGPreflightScreenCaptureAccess`でOS permissionを確認する。frontmost bundle IDを取得し、browser URLも読もうとするが、URL取得失敗時にもcaptureを継続する。private browsing verdict scanを呼ぶ経路は確認できない。

ScreenCaptureKit filterから外すのはAttention自身のwindowである。periodic recorderのexcluded app / window集合とは異なるため、full-display captureにはbackgroundのexcluded windowが写り得る。

結果はJPEG / base64としてRPC responseへ直接返され、通常timelineのframe commit pathには入らない。

## Window-targeted capture

window captureは概ね次の順で対象を選ぶ。

1. bundle IDが一致するwindowをfilter
2. title指定があればexact match
3. bounds指定があれば最も近いCGRect
4. それ以外は最初の候補

この選択はbackground window、excluded app、private browser windowを対象にできる。window IDだけでなく、source PID / bundle / current policyをcapture直前とresponse直前に結び直す必要がある。

## Pauseとoverlay

periodic recorderはpauseやoverlay表示中のrecording gateに従う。一方、manual pathに同じgateを確認できない。

したがって静的解析上は次が可能である。

```text
recordingをpause
  → manual current captureを要求
  → OS permissionだけを通過
  → imageをRPCへ返す
```

overlay自身はcapture filterから除かれるため、overlayを開いた状態のmanual full-display captureでは、その背後のcontentが取得され得る。これはruntimeで漏洩を実証した結論ではなく、共有policy callが見つからないことからの強い推定である。

## History image

`query image`は保存済みframeをIDまたは時刻で探し、必要ならcropしてPNG / base64を返す。これはcurrent captureではなく`history_read`である。

確認できないもの:

- 現在のpause状態
- 現在のapp / domain exclusion
- 現在のScreen Recording permission
- retroactive exclusion
- privacy delete後のknown frame ID invalidation

現在除外したappの古いframeでも、frameがDB / mediaに残りIDが既知なら取得できる可能性がある。これはread pathとretention pathを組み合わせた強い推定であり、runtime fixtureで確認が必要である。

## なぜ危険か

UIのrecording表示やpauseがperiodic pathだけを制御すると、ユーザーが理解するprivacy stateと実際のcapabilityが一致しない。

```text
             periodic    manual live    history image
pause           deny       allow?          allow?
app exclude     deny       allow?          allow?
private deny    deny       allow?          allow?
OS permission   deny       deny             不要?
```

ここで`allow?`は共有deny callを静的に確認できなかった意味で、runtime behaviorの断定ではない。

## OpenBriefの共通PolicyGate

全capture sourceを同じtyped requestへ落とす。

```rust
enum CaptureOrigin {
    Periodic,
    Manual,
    Agent,
}

struct CaptureIntent {
    origin: CaptureOrigin,
    capability: Capability,
    source: SourceIdentity,
    privacy_epoch: u64,
}

enum Capability {
    LiveScreenRead,
    HistoryRead,
}
```

`PolicyGate`は少なくとも次の二地点で同じintentを検証する。

```text
reservation:
  enabled、pause、lock、idle policy、app / domain / private verdict
  source PID / window、capability grant、privacy epoch

before response or commit:
  current epoch、current source identity、current capability
  delete / exclusion / lock transition
```

manual captureを「保存しないから安全」と特別扱いしない。pixelがprocess boundaryやLM Studioへ出る時点でprivacy eventである。

`history_read`は別capabilityとし、次を要求する。

- query可能なtime rangeとresult countの上限
- evidence levelの明示
- excluded intervalを返さない
- retroactive purge後はknown IDでもnot found
- Agentによるimage取得をaudit eventへ残す
- raw imageをstdoutやlogへ暗黙に出さない

MVPではmanual current captureとhistory imageを公開commandにしない。ただし内部`PolicyGate` testでは、将来sourceを追加してもperiodicと同じdeny結果になることを先に固定する。

## Fail-closed fixtures

1. pause中にperiodic / manual / Agent captureを要求。期待は全て`paused`でdeny。
2. excluded foreground上でmanual full-display capture。期待はimage生成前に`app_excluded`。
3. allowed foregroundの背後にexcluded window。期待はwindow filterで除外できなければfull-display capture全体をdeny。
4. private verdictが`Unknown`のbrowser windowを明示指定。期待はdeny。
5. overlay表示中にmanual capture。期待は`recording_gate_closed`。
6. request開始後にpause / exclusion / lock。期待はresponseとcommitを破棄。
7. retroactive purge後にknown frame IDをquery。期待はmetadata、image、cropの全てがnot found。
8. `live_screen_read`だけを持つAgentがhistory imageを要求。期待はcapability denied。

## Runtimeで残る確認

- pause中の`grab-screen`
- excluded / private windowの明示capture
- excluded background windowを置いたfull-display capture
- overlay表示中の背面capture
- Screen Recording permission revoke後のhistory image
- exclusion追加またはdelete後のknown frame ID
- socket clientのcaller authenticationとcapability分離
