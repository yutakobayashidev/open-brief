# 06. Security、privacy、解析限界

## クリーンルーム方針

### 利用する情報

- capture、policy、context、storageを分離する一般的な責務
- bounded queue、ordering、circuit breakerなどの一般的な信頼性pattern
- app/domainをcapture前に除外するprivacy原則
- 公開OS APIと外部crateを用いた独自実装
- 独自に設計したschema、CLI、UX

### 利用しない情報

- Attentionの復元コード、逆アセンブル、逆コンパイル結果
- UI、文言、icon、色、animationの複製
- private endpoint、credential、署名情報
- license、認証、課金、permissionの回避
- Attention固有class名やprivate protocolをOpenBrief APIとして再現すること

本レポートには観測したsymbol、SQLの要約、logの意味だけを記録する。binaryと生成物はrepositoryへ置かない。

## Attentionについて確認できるprivacy境界

**確認**:

- bundle IDとdomainの手動除外
- exclusion groupと自動除外
- focused app/windowが除外ならcaptureをskip
- 除外appのwindowをScreenCaptureKit filterから外す
- 除外appではAX observationもskip
- messaging app除外時にnotification bannerを自動除外するrule
- userによるpauseとpause deadline
- inactivity時のpauseまたはinactive marking
- retention periodとactionの設定

これらの存在は、全pathで漏えいが起きないことを証明しない。特にwindow transition中、notification、overlay、multi-monitor、browser URL取得失敗時の動的挙動は未検証である。

`SelectionCaptureService`には選択textの先頭を含むformat logが存在する。productionでのlog levelと保存先は未確認だが、OpenBriefではselected text、OCR、window titleをdebug logにも含めない。

## Personal memoryとenterprise insightを分ける

提供された利用談ではCoast Localを個人のmemory、Attention cloudをenterprise insightとして説明している。前者の有用性が、後者の収集を自動的に正当化するわけではない。

OpenBriefはMVPとMVP後のEvidence Storeを端末local・本人用に限定する。組織集計、管理者dashboard、従業員比較、productivity scoreは現在のscopeへ入れない。

## Raw dataの範囲

schemaと`storeFrame(...)`から、少なくとも次をlocal storageへ保持し得る。

- screenshotまたはvideo frame
- OCR全文と位置
- application、window title、URL/domain
- 全windowのbounds、layer、z-order
- Accessibility tree
- cursorとmouse button metadata
- inactive状態

これは非常に高感度なactivity logである。OpenBriefはraw screenshot、OCR全文、AX treeをMVPで永続化しない。

## Telemetryについて

binaryにはSentryとTelemetryDeckに関するclass、設定、event名が含まれる。静的文字列の存在だけでは、どのeventがproductionで有効か、本文・画像・URLが送信されるかは判断できない。

そのため本レポートでは「analytics SDKが存在する」以上の主張をしない。確認にはruntime trafficと公開privacy policyの別調査が必要である。

## Agentとlocal IPCの境界

Coast CLI bridgeはApplication Support配下のUnix-domain socketへbindし、newlineで区切ったJSON-RPC 2.0を処理する。request parseからrouter dispatchまでにapplication layerのtoken、handshake、peer credential checkはない。socket作成時の明示的な`chmod 0600`も確認できず、保護は親directoryとsocketのruntime permissionへ依存する。

Agent skillは高感度なactivity dataへ到達するcommandをAgentに教える。localであることだけでは、prompt injection、悪意あるproject instruction、別user process、意図しない大量queryを防げない。

OpenBrief MVPではlistenerを持たず、CLI processがread-only DB projectionを直接queryする。将来IPCを追加する場合は、親directory `0700`、socket `0600`、同一UID peer check、payload上限、rate limit、query audit、write method不在を検証する。

## 静的解析の限界

- optimized Swift binaryでは元の変数名、generic型、async制御flowが失われる
- reflection metadataにclass/function名があっても、production pathで使用されるとは限らない
- log文字列はerror pathの存在を示すが、そのpathが到達可能とは限らない
- feature flag、edition、account stateで挙動が変わり得る
- capture interval、queue上限、debounce値など一部の実値は確定できない
- screen lock、sleep/wake、multi-monitor切替の完全な処理は確認できていない
- binary versionとhashがないため、将来の別buildと厳密比較できない
- 提供されたAgent利用談とartifact recovery例の再現性・一般性は未確認
- CLI socketと親directoryのruntime mode、別layerのpeer check、完全なRPC schemaは未確認
- Agent skillの全文と、各Agentがいつ自律的にqueryを選ぶかは未確認

## OpenBrief向けの追加要件

- capture eligibilityを画像取得前に決める
- raw imageをlog、panic、retry queue、diskへ出さない
- AX enrichmentは別opt-inにする
- excluded期間を空白ではなく理由付きgapとして残す
- model timeout後のlate responseをcommitしない
- pause/delete後はgenerationを更新する
- retention deleteの件数が期待範囲外なら停止する
- timelineからmodel推定と観測時刻を区別できるようにする

## 法的注意

本章は法的助言ではない。製品化前に、対象binaryの利用規約、著作権、相互運用例外、対象地域のprivacy法を専門家と確認する。OpenBriefはAttentionの互換実装ではなく、公開OS APIと一般的な設計patternから独自実装する。
