# 10. Usage timeとsession semantics

## 最大の発見

Attentionの二つのusage APIは、同じ「利用時間」を表していない。

```text
usage.time.recordedSeconds = frameCount × 2秒
usage.sessions.duration    = (lastTimestamp - firstTimestamp) / 1000
```

例:

```text
frames = [0ms, 2000ms, 4000ms]

usage.time     = 3 frames × 2秒 = 6秒
session length = (4000 - 0)     = 4秒
```

OpenBriefはこの違いをコピーせず、sample countとactive durationを別metricにする。

## `usage.time`

### Range

入力はepoch millisecondsで、時間範囲は半開区間である。

```text
[start_ms, end_ms)
```

最初にtimestamp条件からmin/max frame IDを解決する。

### Count

countは実frame rowの`COUNT(*)`ではなく、segmentの`start_frame_id`と次segmentの開始IDからspanを計算する。

```sql
SUM(MIN(next_start, range_end) - MAX(start_frame_id, range_start))
```

application/domain filterがある場合は対応tableをjoinする。両方を指定するとAND条件であり、filter値はlowercase化される。

### Seconds

`0x1001be41c`はcountを受け取り、次を返す。

```text
frameCount      = count
recordedSeconds = count << 1
```

固定2秒cadenceをAPI semanticsへ埋め込んでいる。

### ID gap

segment spanをID差で数えるため、途中のframe rowが削除されてもcountへ含まれる可能性がある。通常capture dropはINSERT自体がないためID穴になりにくいが、retention、破損、手動削除後は実row数とずれ得る。

## `usage.byApplication`

applicationごとにsegment spanをclipし、frame countを合計する。

```text
group by bundle_id
order by frame_count desc
limit N
```

display nameは`MAX`を使う。同数時の明示tie-breakはない。`is_inactive`は参照しない。

## `usage.byDomain`

domainごとに同じsegment span集計を行う。

```text
group by domain.id
return normalized_domain + common_name
```

domainのないsegmentは含まれない。inactive判定はない。

## `usage.sessions`

sessionsはusage timeと違い、対象segment内の**実際に存在するframe timestamp**を読む。

### Split

`LAG(timestamp)`で前frameとの差を求め、累積SUMでsession IDを作る。

```text
delta_ms >= gap_seconds × 1000
  → new session
```

gapが0または未指定なら3秒を使う。

| timestamps | default 3秒 | 結果 |
|---|---|---|
| 0, 2999 | threshold未満 | 1 session |
| 0, 3000 | threshold以上 | 2 sessions |
| 0, 4000 | capture一回欠損 | 2 sessions |

2秒captureを前提にした非常に敏感な分割である。

### Duration

session rowは次を返す。

```text
start_ms = MIN(timestamp)
end_ms = MAX(timestamp)
frame_count = COUNT(*)
duration_seconds = integer((end_ms - start_ms) / 1000)
```

- single-frame sessionは0秒
- 1500msは1秒
- last frame後のcapture intervalは加算しない
- total durationは各session durationの単純和

## Inactive、pause、excluded

usage queryはpause eventを直接参照しない。

| 状態 | 結果 |
|---|---|
| inactive frameを保存しflagだけ立てる | usageへ含む |
| inactivityでrecordingをpause | frameがなくusageへ含まない |
| user pause / disk full / overlay pause | timestamp gapとしてだけ見える |
| excluded app/domain | captureされなければgapになる |

schemaに`is_inactive`はあるが、今回確認したusage time、application、domain、sessionsのSQLはfilterしない。

## 日付とtimezone

usage SQLはtimezone変換も日付groupingも行わない。

- epoch millisecondsをtimestamp順に処理
- 日付境界でsessionを強制分割しない
- 23:59:59と00:00:01がgap未満なら同じsession
- timezoneはcallerがstart/endを作る時の責務

日単位queryでは同じsessionがrange boundaryでclipされ、別々の短いspanに見え得る。

## OpenBriefへの採用判断

OpenBriefはforeground eventを持つため、固定`2秒/frame`を使わない。

```text
sample_count
sampled_seconds = sample_count × configured_cadence

active_duration
  = segment durationをquery範囲でclip
  - pause
  - lock
  - inactive interval
  - excluded / unknown gap
```

`sampled_seconds`は観測密度、`active_duration`は時間として別名で返す。model summaryからdurationを推測しない。

最低限のfixture:

1. single frame
2. normal cadence
3. gap直前と境界
4. capture欠損
5. start inclusive / end exclusive
6. inactive flag
7. pause/excluded gap
8. app + domain AND filter
9. 日跨ぎ
10. segment内ID穴

## 主なfunctionとSQL

| Address | 意味 |
|---|---|
| `0x1001be41c` | `usage.time` wrapperと秒換算 |
| `0x1001c1ff4` | timestamp rangeからframe ID rangeを解決 |
| `0x1001c26ac` | usage count query |
| `0x100eb3640` | by-application SQL |
| `0x100eb3470` | by-domain SQL |
| `0x1001c0848` | sessions async入口 |
| `0x1001c1030` | session sync集計とdefault gap |
| `0x1001c3780` | session SQL |
| `0x1001c1100` | row parseとduration |
| `0x100eb3210`–`0x100eb3300` | gap/session CTE |

## 未確認

- by-application/domainのpublic rowでframe countを秒へ変換する正確な箇所
- deleted IDを含むproduction dataでの実挙動
- caller側の日付range生成とtimezone履歴の利用
