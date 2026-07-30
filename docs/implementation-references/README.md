# OSS implementation references

## 目的

OpenBriefの実装中に、同じOSS repositoryを毎回最初から調べ直さないためのsource-levelメモである。

各文書は次を固定する。

- 調査したcommitとlicense
- 有用なsource path
- OpenBriefへ採るpattern
- 直接再利用しない範囲
- 再調査する条件

最新mainへの追従記録ではない。実装判断が変わる条件を満たすまで、記録したcommitを基準にする。

## 文書一覧

| 文書 | 基準 | 一文で言う判断 |
|---|---|---|
| [Screenpipe](01-screenpipe-source-reference.md) | 現行source `d114e14…`、最終MIT `892199f…` | forkせず、旧MITの小moduleだけ条件付き評価 |
| [Entire CLI](02-entire-cli-source-reference.md) | stable `v0.9.0` / `8b77ad4…` | Go codeを依存せず、event・state・CLI patternを採用 |
| [Buzz](03-buzz-source-reference.md) | source `63496cc…` | Tauri process ownershipとACP harness patternだけを採用 |

## 共通ルール

1. branch名ではなくcommit SHAへlinkする。
2. codeを移植する場合は、そのblobのlicenseをcommit時点で再確認する。
3. repository全体をdependencyまたはforkにしない。
4. OpenBriefの小traitへ切り直し、upstreamの大きなdomain modelを持ち込まない。
5. sourceを見たことと、OpenBriefで有効性を検証したことを混同しない。
