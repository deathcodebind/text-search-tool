# 全文搜索竞标助手 接口契约草案（MVP）

## 1. 目标与范围

本契约用于统一以下模块边界：
- gui（Tauri + Vue3 + Pinia）
- crawler
- storage
- search-engine

MVP 原则：
- 先保证端到端可跑通。
- 接口语义稳定优先于字段完备。
- 字段可追加但尽量不做破坏性变更。

## 2. 通用约定

### 2.1 时间与时区
- 统一使用 UTC 存储。
- 对外传输使用 ISO 8601 字符串，例如 2026-05-19T08:30:00Z。

### 2.2 ID 与唯一性
- record_id：系统内部主键（UUID v7 或雪花 ID，二选一）。
- source_record_id：来源网站记录 ID（去重主键）。
- source_url：来源 URL（去重辅键）。
- rule_id、job_id：UUID。

### 2.3 分页与排序
- 分页参数：page（从 1 开始）、page_size。
- 排序参数：sort_by、sort_order（asc/desc）。
- 统一返回 total、page、page_size、items。

### 2.4 错误返回
统一错误结构：

```json
{
  "code": "CRAWLER_AUTH_EXPIRED",
  "message": "登录已过期，请重新登录",
  "details": {
    "source": "jxemall",
    "job_id": "..."
  }
}
```

错误码前缀约定：
- COMMON_
- GUI_
- CRAWLER_
- STORAGE_
- SEARCH_

## 3. 核心数据模型

### 3.1 竞标记录 Record

```json
{
  "record_id": "uuid",
  "source": "jxemall",
  "source_record_id": "JXE-123456",
  "source_url": "https://...",
  "title": "某项目招标公告",
  "content": "正文文本...",
  "region": "江西省",
  "published_at": "2026-05-19T01:20:00Z",
  "deadline_at": "2026-05-25T10:00:00Z",
  "extra": {
    "budget": "1000000",
    "category": "信息化"
  },
  "created_at": "2026-05-19T08:30:00Z",
  "updated_at": "2026-05-19T08:30:00Z",
  "expired": false
}
```

说明：
- extra 为站点特有字段扩展区，MVP 允许弱约束。
- content 为搜索主文本，可由多字段拼接后清洗生成。
- jxemall 映射约定：`source_record_id = requisitionId(string)`，`published_at = pubTimestamp(ms) -> UTC`。
- `requisitionId` 全链路按字符串处理，不做数值化转换。
- 返回值中的 trade 系列字段不参与 `content` 拼接。

MVP 字段白名单（Record）：
- 主字段：record_id/source/source_record_id/source_url/title/content/region/published_at/deadline_at/created_at/updated_at/expired
- extra 白名单：biddingId/requisitionId/startTimestamp/remainingMilliseconds/state/districtCode/orgName/budget/dealAmount/type/categoryType/categoryTypeText
- source_url 在详情接口未接入前允许使用占位值（例如 about:blank 或空字符串），后续回填并触发更新。
- 已确认详情链接模板：`/luban/bidding/detail?requisitionId={requisitionId}&type={type}`。
- `source_url` 生成时忽略 `utm` 等跟踪参数，避免去重污染。
- `budget` / `dealAmount` 在 MVP 阶段暂按“元”解释。
- `subState` 暂不纳入 MVP 持久化模型。
- 返回值中的 `tradeModel` / `tradeStyle` / `displayTradeStyle` 当前不纳入 MVP 持久化模型，只保留在原始响应层。

### 3.2 关键词规则 Rule

```json
{
  "rule_id": "uuid",
  "name": "弱电项目筛选",
  "enabled": true,
  "query": {
    "must": [
      { "term": "弱电", "boost": 2.0 }
    ],
    "should": [
      { "term": "安防", "boost": 1.2 },
      { "term": "综合布线", "boost": 1.1 }
    ],
    "must_not": [
      { "term": "家具" }
    ],
    "minimum_should_match": 1
  },
  "score_weights": {
    "a_keyword": 0.6,
    "b_field": 0.3,
    "c_freshness": 0.1
  },
  "created_at": "2026-05-19T08:30:00Z",
  "updated_at": "2026-05-19T08:30:00Z"
}
```

### 3.3 抓取任务 CrawlJob

```json
{
  "job_id": "uuid",
  "source": "jxemall",
  "status": "running",
  "window_start": "2026-05-18T09:00:00Z",
  "window_end": "2026-05-19T09:00:00Z",
  "progress": {
    "fetched": 120,
    "upserted": 98,
    "deduplicated": 22,
    "indexed": 98
  },
  "error": null,
  "started_at": "2026-05-19T09:00:01Z",
  "finished_at": null
}
```

状态枚举：
- pending
- running
- succeeded
- failed
- canceled

## 4. GUI <-> Tauri Command 契约

说明：MVP 建议通过 Tauri command 作为统一入口，由 command 调用 Rust 各 crate 服务。

### 4.1 凭证与会话

1) save_credential
- 入参：

```json
{
  "source": "jxemall",
  "username": "xxx",
  "password": "***"
}
```

- 出参：

```json
{
  "credential_ref": "cred://jxemall/default"
}
```

2) validate_session
- 入参：

```json
{
  "source": "jxemall",
  "credential_ref": "cred://jxemall/default"
}
```

- 出参：

```json
{
  "valid": true,
  "expires_at": "2026-05-19T12:00:00Z"
}
```

### 4.2 抓取任务

1) start_crawl
- 入参：

```json
{
  "source": "jxemall",
  "credential_ref": "cred://jxemall/default",
  "filters": {
    "backCategoryName": "",
    "tradeModel": "BIDDING",
    "categoryType": "GOODS",
    "pageNo": 1,
    "pageSize": 16,
    "stateList": [4],
    "otherSearch": "",
    "minBudget": 50000,
    "maxBudget": 60000,
    "instanceCodes": ["JXWSCS", "JXDDCG", "JXXYGH", "JXFWGC"],
    "sortField": "ANNOUNCEMENT_PUBLISH_TIME",
    "sortMethod": "DESC",
    "districtCodeList": [],
    "administrativeDistrictCodeList": []
  },
  "window": {
    "start": "2026-05-18T09:00:00Z",
    "end": "2026-05-19T09:00:00Z"
  }
}
```

说明：
- `filters` 为站点请求参数透传对象，当前以上字段已由抓包样本确认。
- 后续若站点参数扩展，采用可选字段追加，不做破坏性修改。
- listNewest 调用方式已确认：`POST + application/json`。
- listNewest 请求头最小集：`Content-Type`、`Cookie`、`Origin`、`Referer`、`X-Requested-With`。
- BIDDING/REVERSE 请求头同构；`Referer` 路径分别对应 `/luban/bidding/newest` 与 `/luban/reverse/newest`。
- `categoryType` 在“全部”场景下需显式传 `null`。
- 建议在内部使用中间枚举，再序列化为 `GOODS`/`SERVICE`/`PROJECT`，降低语义歧义。
- 语义权威以页面中文筛选项为准：`PROJECT` 在本项目中映射“工程类”。
- `stateList` 同样建议由中间语义枚举（全部/未开始/竞价中/已过期）映射生成，不在业务层直接写状态码。
- 控制总价使用 `minBudget`/`maxBudget`，选择“全部”时省略两字段（不要传 `null`）。
- 地区筛选当前以 `districtCodeList` 为主；`administrativeDistrictCodeList` 固定传空数组。
- 无地区筛选：`districtCodeList=[]` 且 `administrativeDistrictCodeList=[]`。
- 有地区筛选：`districtCodeList=[code...]` 且 `administrativeDistrictCodeList=[]`。
- `tradeModel` 为隐藏筛选项/模块切换参数；当前已知值至少有 `BIDDING` 与 `REVERSE`。
- `tradeModel` 是模块选择主参数（竞价/反拍）；同一路由通过该字段切换数据域。
- `categoryType` 是业务类目维度（`GOODS`/`SERVICE`/`PROJECT`/`null`），不与 `tradeModel` 混用。
- `type` 在当前协议中主要体现为响应字段/详情参数，不作为 listNewest 主筛选入参。
- REVERSE 无筛选样本 filters：`tradeModel=REVERSE`、`categoryType=null`、`stateList=[3,4,5,6,7,10,12,50]`、`instanceCodes=["JXWSCS"]`。
- 排序枚举已确认：`sortField` ∈ {`ANNOUNCEMENT_PUBLISH_TIME`, `QUOTE_DEADLINE`, `BUDGET_AMOUNT`}，`sortMethod` ∈ {`ASC`, `DESC`}。
- 请求依赖登录态 Cookie；command 层仅传 `credential_ref`，不传明文 Cookie。
- crawler 侧可记录 Cookie 键名存在性用于诊断，但不得记录 Cookie 值。
- 当地区筛选结果异常时，优先排查 Cookie 中 `districtCode/districtType` 与 body 地区参数的组合影响。
- `Host`/`Content-Length` 由客户端自动处理，不在业务层手动拼装。

- 出参：

```json
{
  "job_id": "uuid",
  "status": "pending"
}
```

2) get_crawl_job
- 入参：

```json
{
  "job_id": "uuid"
}
```

- 出参：CrawlJob

3) list_crawl_jobs
- 入参：分页参数
- 出参：分页后的 CrawlJob 列表

### 4.3 规则管理

1) create_rule
2) update_rule
3) delete_rule
4) list_rules
5) toggle_rule

说明：create/update 入参为 Rule（不含 created_at、updated_at）。

### 4.4 检索与详情

1) search_records
- 入参：

```json
{
  "rule_id": "uuid",
  "paging": {
    "page": 1,
    "page_size": 20
  },
  "sort": {
    "sort_by": "score",
    "sort_order": "desc"
  }
}
```

- 出参：

```json
{
  "total": 120,
  "page": 1,
  "page_size": 20,
  "items": [
    {
      "record_id": "uuid",
      "title": "某项目招标公告",
      "region": "江西省",
      "published_at": "2026-05-19T01:20:00Z",
      "source_url": "https://...",
      "highlights": ["...弱电..."],
      "score": 12.34,
      "score_breakdown": {
        "keyword": 8.0,
        "field": 3.0,
        "freshness": 1.34
      }
    }
  ]
}
```

2) get_record_detail
- 入参：

```json
{
  "record_id": "uuid"
}
```

- 出参：Record

jxemall 实现说明（补充）：
- 优先通过详情 API 拉取正文：
  - `/api/sparta/announcement/detail?requisitionId={requisitionId}&type={type}&timestamp={timestamp}`
- 请求方法：`GET`
- 请求头最小集：`Accept`、`Cookie`、`Referer`、`X-Requested-With`
- `record_id` -> `requisitionId/type` 的映射由 storage 层维护。
- `timestamp` 由 crawler 运行时动态生成。
- 响应结构：`success/result/code/message`，仅在 `success=true` 且 `result` 非空时进入映射流程。
- 详情字段映射：`result.title`、`result.content`、`result.state`、`result.releasedAt`、`result.attachments[]`、`result.purchaserOrgName`。
- `result.content` 为 HTML，需产出 `content_html`（回显）与 `content_text`（检索）。
- `releasedAt` 按 `Asia/Shanghai` 解析后统一转 UTC。
- `requisitionId` 一致性校验来自 `result.relateAnnouncementTypes[].requisitionId`（string 全等）。

## 5. crate 内部服务契约（建议）

### 5.1 crawler -> storage

1) upsert_records(records: Vec<Record>) -> UpsertResult
- 返回：inserted、updated、deduplicated。

2) create_crawl_job(job: CrawlJobCreate) -> job_id

3) update_crawl_job_progress(job_id, progress)

4) finish_crawl_job(job_id, status, error?)

### 5.2 storage -> search-engine

1) reindex_records(record_ids: Vec<String>)
- 用于抓取后的增量索引更新。

2) remove_records(record_ids: Vec<String>)
- 用于过期清理后的索引删除。

### 5.3 search-engine

1) execute_rule_search(rule: Rule, paging, sort) -> SearchPage

2) explain_score(record_id, rule_id) -> ScoreBreakdown

## 6. 调度与后台任务契约

### 6.1 每日增量抓取
- 调度器每天固定时刻触发。
- 自动计算 window_start/window_end。
- 上一次成功任务时间作为下一次起点。

### 6.2 3 天过期清理
- 每日固定时刻执行。
- 筛选 published_at 或入库时间超过 3 天的记录。
- 先删除元数据，再触发索引删除。

## 7. 并发与幂等

- start_crawl 同 source 在 running 状态下默认拒绝重复启动。
- upsert_records 必须幂等（jxemall 默认按 source + requisitionId，若 type 非空则按 source + requisitionId + type 分流）。
- `requisitionId` 比较使用字符串全等，禁止先转整型再比较。
- reindex_records 可重复调用且不产生重复文档。

## 8. 错误码建议

- COMMON_INVALID_ARGUMENT
- COMMON_NOT_FOUND
- COMMON_INTERNAL
- CRAWLER_AUTH_EXPIRED
- CRAWLER_RATE_LIMITED
- CRAWLER_SITE_CHANGED
- STORAGE_CONFLICT
- STORAGE_WRITE_FAILED
- SEARCH_QUERY_INVALID
- SEARCH_INDEX_UNAVAILABLE

## 9. 版本与变更策略

- 契约版本：v0（MVP）
- 兼容策略：
  - 新增字段使用可选字段。
  - 删除字段需升级版本并提供迁移说明。

## 10. 首批落地建议

1. 先实现最小命令集：save_credential、start_crawl、get_crawl_job、create_rule、search_records、get_record_detail。
2. 首批仅做同步调用 + 轮询任务进度，不引入复杂消息总线。
3. 每个命令先补一条成功路径集成测试，再补失败路径。
