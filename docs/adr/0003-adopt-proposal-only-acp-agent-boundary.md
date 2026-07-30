# ADR 0003: Proposal-only ACP Agent boundaryを採用する

## Status

- Accepted: 2026-07-30

## Decision Drivers

- Brief生成はstateful Agentへ委任しつつ、Agent sessionをOpenBriefのauthorityにしたくない
- 自然言語triageを使いたいが、model解釈を本人の確定判断として自動保存したくない
- Codex、Hermes、OpenClaw等を将来交換できるcontrol planeが必要である
- Gmail、Slack、画面等の集約contextを無制限にAgentへ公開したくない
- LM Studioは画面観測用VLMであり、Brief生成runtimeと混同しない

## Context

OpenBriefはObservation、Agentの提案、本人が確定した判断を別の型とtableで保持します。Desktopは一画面の有限Brief、自然言語triage、常時見えるReturn Threadを提供します。

最初のinteractive runtimeにはCodex ACP adapterを使います。HermesのcronやGmail / Slack toolはscheduled Observation producer候補ですが、SSH設定が必要なためこのMVPの起動条件にはしません。

## Options Considered

- Agentを直接使い、chat memoryをauthorityにする: Agent交換とsession終了で判断が失われるため採用しない
- OpenAI-compatible APIだけを共通interfaceにする: streaming、tool、permission、cancelのsession lifecycleを表現できないため採用しない
- MCP toolからUserDecisionを直接作る: model提案と本人判断が混ざるため採用しない
- OpenBriefが独自の推論loopを持つ: Agentの既存能力を重複実装するため採用しない
- ACPでAgentを所有し、MCPはproposal-onlyにする: 採用する

## Decision

DesktopのRust processが公式ACP Rust SDKで、選択されたlocal ACP runtimeを一つだけ起動します。Buzzのruntime管理から、provider ID、表示名、同梱path、引数、version policy、認証方法の優先hint、OpenBrief MCPの付与可否をdataとして持つ静的catalogだけを採用します。共通のresolverとprobeがcatalog entryを汎用`AgentConfig`へ変換し、ACP transportにはprovider別traitを導入しません。選択providerが変わった場合は既存runtimeを再利用せず、停止後に新しいentryを起動します。

最初のcatalog entryはOpenBrief packageに固定された`codex-acp`です。Nix packageでは`$out/libexec/openbrief/codex-acp`、通常のDesktop packageでは同じversionのTauri resourceを内部解決します。configの`agent.provider`は`codex`を既定とし、`agent.executable_path`はadvanced overrideだけに使います。ACPはinitialize、session、prompt stream、permission、cancel、process cleanupを担当します。

第二のentryとして固定版`pi-acp`を提供します。Piも同じACP runtimeを使い、provider固有clientは追加しません。ただし`pi-acp` v0.0.33はACP MCP設定をPiへ転送しないためOpenBrief MCPを付与せず、会話、stream、tool表示、cancelだけを対応範囲とします。Piのterminal authは事前設定を要求し、ACP `authenticate`の成功をlogin完了とは扱いません。

OpenBriefは同じCLI binaryのhidden `openbrief mcp serve`をstdio MCP serverとしてsessionへ渡し、次の二toolだけを公開します。

- `brief_propose`: 最大3件の根拠付き有限Briefをinert proposalとして保存する
- `triage_propose`: 自然言語triageの解釈をinert proposalとして保存する

MCP toolは`UserDecision`、`CuriosityCapture`、`ReturnAnchor`を作れません。Desktopで本人が提案を確認したときだけ、application serviceが選択要素を一transactionで確定します。

Agentへ渡すsnapshotは最新ObservationBatchを最大100件、64 KiBまでに制限します。source contentはuntrusted evidenceとして明示し、未知のtool requestとexternal writeは拒否します。proposal toolの`allow_once`だけをruntimeが選択できます。

Agent executableは選択されたcatalog entryの固定package resourceまたは明示的な絶対path overrideからだけ起動します。runtime download、PATH探索、`npx -y`、別Agentへの自動fallback、user定義dynamic harnessは行いません。availability、authentication、process stateは別fieldで同期的に返し、認証方法はACP initialize responseから取得します。

LM Studioは将来の`observe_frame` producer用Model Gatewayとして扱い、このBrief生成経路には入れません。scheduled producerもDesktop ACP sessionから分離します。

## Consequences

- Positive: Agentを交換・終了しても、Observation、proposal、本人判断、Return AnchorがSQLiteに残る
- Positive: 自然言語UIとlocal authorityを両立し、Agentのexternal action能力をMVPから排除できる
- Positive: Buzz全体をforkせず、process ownership patternだけを小さく再実装できる
- Positive: 第二のACP runtimeはtransportやUI state machineを複製せず、catalog entryと配布物を追加できる
- Positive: Nix利用者も通常のDesktop利用者もadapterを手動installせず、同じversionとonboardingを使える
- Negative: package sizeは固定Node版`codex-acp`と内蔵Codexの分だけ増える
- Negative: bounded snapshotを超える検索、複数session、scheduled executionはまだ提供しない
- Negative: Agentへ渡すObservationは、選択したAgent/providerのprivacy policyに従って処理される
- Negative: provider固有の高度な設定やmanaged installはcatalogでは表現せず、必要性が確認されるまで対応しない
- Follow-up: fake ACP processによるcancel / timeout / cleanup回帰testを追加する
- Follow-up: Hermes scheduled producerはObservation ingress contractだけを使って別途接続する

## Adoption and Exceptions

- MCP tool一覧のtestは`brief_propose`と`triage_propose`以外を拒否する
- domain / store testはproposal作成だけで本人状態が増えないことを確認する
- reviewではAgent出力から`UserDecision`等を直接作る経路を認めない
- 新しいACP runtimeは静的catalog entry、配布物、resolver/probe testを一組で追加する
- 新しいAgent runtime、MCP write tool、snapshot上限変更、off-device egressは別ADRまたは本ADRの更新を要求する

例外はrepository maintainerが、公開data、permission、失敗時挙動、本人確認方法を示すtestとともに承認します。
