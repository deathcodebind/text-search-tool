# jxemall listNewest 数据画像与映射规范（MVP）

## 1. 数据来源

- Endpoint: https://www.jxemall.com/api/sparta/announcement/listNewest
- 请求方法：POST（已确认）。
- Content-Type：application/json（已确认）。
- 响应包结构：
  - success: bool
  - result: object
  - code: nullable
  - message: nullable

说明：当前样本中 `result.total` 有值，但分页相关字段（pageNum/pageSize/pages 等）均为 0，不能假定其分页语义稳定。

## 2. 列表项字段画像

样本字段（单条 list item）：
- announcementId: nullable
- biddingId: number
- requisitionId: string
- title: string
- pubTimestamp/startTimestamp/endTimestamp: epoch milliseconds
- remainingMilliseconds: number
- state/subState: number
- districtName/districtCode: nullable string
- orgName: string
- budget/dealAmount: nullable number
- type/tradeModel/tradeStyle/displayTradeStyle: string
- provinceName/cityName/areaName: nullable string
- categoryType/categoryTypeText: number/string
- 其他业务展示字段若干（collectionStatus 等）

当前样本补充观察：
- 当前给出的列表样本中，`type` 均为 `BIDDING_INVITATION`。

## 3. MVP 定版内部映射（到 Record）

内部标准字段（定版）：
- source = "jxemall"
- source_record_id = requisitionId（详情关联主键）
- requisitionId 类型约束：string（入库、去重、拼 URL/API 参数时均按字符串处理）
- source_url = 详情页 URL（需后续确认拼接规则或额外接口获取）
- title = title
- content = 由 title + orgName + districtName + provinceName + cityName + categoryTypeText + type 拼接得到
- region = districtName（可回退 province/city 组合）
- published_at = pubTimestamp 转 UTC
- deadline_at = endTimestamp 转 UTC
- extra = 固定白名单子集（见下）

`extra` 白名单（MVP 定版）：
- biddingId
- requisitionId
- startTimestamp
- remainingMilliseconds
- state
- districtCode
- orgName
- budget
- dealAmount
- type
- categoryType
- categoryTypeText

### 3.1 content 拼接规则（MVP 定版）

按以下顺序拼接非空字段，并以单空格分隔：
1. title
2. orgName
3. districtName
4. cityName
5. provinceName
6. categoryTypeText
7. type

清洗规则：
- 去除首尾空白。
- 连续空白折叠为单空格。
- 保留中文与数字原样，不做大小写改写。

### 3.2 Record 字段白名单（MVP 定版）

持久化主字段：
- record_id
- source
- source_record_id
- source_url
- title
- content
- region
- published_at
- deadline_at
- created_at
- updated_at
- expired

索引核心字段：
- title
- content
- region
- published_at

## 4. 去重与幂等建议

- 主键：`source + requisitionId`
- 冲突分流键：`source + requisitionId + type`（仅当 `type` 非空时参与）
- 辅助键：`source + biddingId`
- 若后续拿到稳定详情 URL，再追加 `source + source_url` 作为辅助去重键。
- 注意：`requisitionId` 为字符串主键，比较时使用字符串全等，不做整数化。

Upsert 规则：
1. 命中 `source + requisitionId` 则更新可变字段（title、budget、state、deadline 等）。
2. 当同一 `requisitionId` 出现多个 `type` 时，按 `source + requisitionId + type` 分流存储。
3. 若 `requisitionId` 缺失（理论上少见）回退 `source + biddingId` 匹配。
4. 保留 `first_seen_at` 与 `last_seen_at`，用于审计与增量稳定性分析。

## 4.1 详情页 URL 规则（已确认）

已确认页面链接模板：
- `https://www.jxemall.com/luban/bidding/detail?requisitionId={requisitionId}&type={type}`

样本：
- `https://www.jxemall.com/luban/bidding/detail?requisitionId=62026052077992411&type=BIDDING_INVITATION&utm=...`

规则说明：
- 生成 `source_url` 时使用 `requisitionId + type` 作为主要参数。
- `utm` 属于跟踪参数，存储与去重时应去除。
- 若 `type` 缺失，先回退仅使用 `requisitionId` 生成 URL，并打调试日志。

## 4.2 详情正文 API 规则（已确认）

已确认详情数据由 API 渲染：
- `https://www.jxemall.com/api/sparta/announcement/detail?requisitionId={requisitionId}&type={type}&timestamp={timestamp}`

样本：
- `https://www.jxemall.com/api/sparta/announcement/detail?requisitionId=62026052077992411&type=BIDDING_INVITATION&timestamp=1779256912`

参数说明：
- `requisitionId`：详情主查询键（来自列表项）。
- `type`：当前样本为 `BIDDING_INVITATION`。
- `timestamp`：防缓存/时效参数（当前样本为 10 位整型，疑似秒级时间戳）。

请求方法（已确认）：
- `GET`

实现建议：
- 优先走详情 API，不走详情页 HTML 解析。
- `timestamp` 由客户端按当前时间动态生成，避免硬编码常量。
- 当详情 API 失败时，可降级保留列表字段并记录重试任务。

## 4.3 详情 API 返回体映射与正文抽取（已确认）

响应外层结构：
- `success: bool`
- `result: object`
- `code: nullable`
- `message: nullable`

`result` 关键字段映射（MVP）：
- `result.relateAnnouncementTypes[].requisitionId` -> 与列表项 `requisitionId` 交叉校验（string 全等）。
- `result.relateAnnouncementTypes[].announcementTypeEnum` -> 详情请求中的 `type` 值候选（如 `BIDDING_INVITATION`）。
- `result.title` -> 标题补正来源（优先级高于列表标题）。
- `result.content` -> 详情正文 HTML 原文（建议保存原文与提纯文本两份）。
- `result.state` -> 状态补正字段（可覆盖列表状态）。
- `result.releasedAt` -> 发布时间候选（格式 `yyyy-MM-dd HH:mm:ss`，按 `Asia/Shanghai` 解析后转 UTC 存储）。
- `result.remainingMilliseconds` -> 动态字段，仅用于展示，不作为持久化主语义字段。
- `result.purchaserOrgName` -> 机构名补正来源。
- `result.attachments[]` -> 附件元数据集合（`attachmentName`、`attachmentPath`、`size`、`url`）。

正文抽取规则（MVP）：
1. 输入使用 `result.content`（HTML 字符串）。
2. 清洗时移除 `<style>`、`<script>` 标签内容。
3. 保留段落和表格文字，按块换行拼接为纯文本。
4. HTML 实体解码后折叠连续空白为单空格，保留中文标点。
5. 生成 `content_text`（检索用）与 `content_html`（回显用）双轨字段。

附件处理规则（MVP）：
- 附件不做正文 OCR/解析，仅保存结构化元数据。
- `attachments[].url` 作为可点击外链；失败不影响主流程。

一致性校验规则（MVP）：
- 若 `result.relateAnnouncementTypes` 中不存在与请求 `requisitionId` 一致的项，记诊断日志并标记异常样本。
- 若 `title/state/releasedAt` 与列表差异，详情值优先，并保留原列表值用于审计（可选存 `raw_snapshot`）。

详情 API 请求头最小集（已确认）：
- `Accept: application/json, text/plain, */*`
- `Cookie: ...`（需包含登录态与租户键）
- `Referer: https://www.jxemall.com/luban/bidding/detail?requisitionId={requisitionId}&type={type}`
- `X-Requested-With: XMLHttpRequest`

详情 API 可选增强头：
- `User-Agent`

详情 API 浏览器噪声头（MVP 可不强依赖）：
- `Sec-Fetch-*`
- `sec-ch-ua*`
- `Accept-Language`
- `Accept-Encoding`
- `Connection`

## 5. 时间与数值规范

- 时间字段统一按 epoch ms -> UTC ISO8601 存储。
- `budget` 当前按“元”解释与展示；若后续发现异常样本再回滚修正。
- `dealAmount` 当前按“元”解释与展示；可空，空值不参与排序/统计。

## 5.1 当前跳过字段

- `subState` 含义暂不清楚，且与当前正文抓取/检索链路无明显关联。
- MVP 阶段不将 `subState` 纳入持久化主模型与筛选模型；如后续发现它影响可投标判定，再单独补回。

## 6. 增量抓取策略（列表接口）

目标：每日固定时刻抓取前次成功时刻至当前时刻的数据。

建议策略：
1. 记录上次成功抓取的 `watermark_pub_ts`（毫秒）。
2. 按接口返回顺序遍历列表，持续拉取直到遇到 `pubTimestamp < watermark_pub_ts` 且连续 N 条（防抖，建议 N=20）。
3. 对于 `pubTimestamp == watermark_pub_ts` 的记录，优先使用 `requisitionId` 做二级比较，缺失时回退 `biddingId`。
4. 每批 upsert 后更新任务进度，不在中途推进 watermark。
5. 任务整体成功后一次性提交新 watermark（事务化更新任务状态）。

注意：若接口分页参数不稳定，则以“时间窗口 + 去重幂等”优先保障正确性。

## 6.1 listNewest 请求规则（v1 已确认）

已确认 payload 形态（BIDDING 样本）：

```json
{
  "backCategoryName": "",
  "tradeModel": "BIDDING",
  "categoryType": "GOODS",
  "pageNo": 1,
  "pageSize": 16,
  "stateList": [4],
  "otherSearch": "",
  "instanceCodes": ["JXWSCS", "JXDDCG", "JXXYGH", "JXFWGC"],
  "sortField": "ANNOUNCEMENT_PUBLISH_TIME",
  "sortMethod": "DESC",
  "districtCodeList": [],
  "administrativeDistrictCodeList": []
}
```

已确认 payload 形态（REVERSE 无筛选样本）：

```json
{
  "backCategoryName": "",
  "tradeModel": "REVERSE",
  "categoryType": null,
  "pageNo": 1,
  "pageSize": 16,
  "stateList": [3, 4, 5, 6, 7, 10, 12, 50],
  "otherSearch": "",
  "instanceCodes": ["JXWSCS"],
  "sortField": "ANNOUNCEMENT_PUBLISH_TIME",
  "sortMethod": "DESC",
  "districtCodeList": [],
  "administrativeDistrictCodeList": []
}
```

参数语义（当前确认）：
- pageNo/pageSize：分页控制参数。
- stateList：状态过滤。
  - 全部：`[3, 4, 5, 6, 7, 10, 12, 50]`
  - 竞价未开始：`[3]`
  - 竞价中：`[4]`
  - 已过期：`[5, 6, 7, 10, 12, 50]`
- sortField/sortMethod：排序控制。
  - `sortMethod`：`ASC` / `DESC`
  - `sortField`（按界面顺序）：
    - `ANNOUNCEMENT_PUBLISH_TIME`（公告发布时间）
    - `QUOTE_DEADLINE`（竞价截止时间）
    - `BUDGET_AMOUNT`（控制总价）
- instanceCodes：业务实例范围过滤。
  - 编码集合固定：`JXWSCS/JXDDCG/JXXYGH/JXFWGC`
  - 全部（按页面顺序）：`["JXWSCS","JXDDCG","JXXYGH","JXFWGC"]`
  - 单项映射：
    - `JXWSCS` -> 江西网上超市
    - `JXDDCG` -> 江西定点采购馆
    - `JXXYGH` -> 江西协议供货馆
    - `JXFWGC` -> 江西服务工程馆
- 地区过滤：`districtCodeList` 承载地区筛选值；`administrativeDistrictCodeList` 当前固定为空数组。
- 已确认“全选地区”语义：`districtCodeList` 为完整地区代码集（非空数组）。
- categoryType：类目过滤。
  - 全部：`null`（需显式传）
  - 货物类：`GOODS`
  - 服务类：`SERVICE`
  - 工程类：`PROJECT`
- type：当前定位为“公告类型/详情路由参数”字段，主要出现在列表响应与详情接口，不作为 listNewest 主筛选入参。
- 当前样本下 listNewest body 未观测到 `type` 入参；模块切换仍由 `tradeModel` 决定。
- tradeModel：隐藏筛选项/模块切换参数。
  - 当前已知值：`BIDDING`、`REVERSE`
  - 同一接口可通过不同 `tradeModel` 拉取不同模块数据。
- backCategoryName/otherSearch：文本或类目补充过滤，样本为空。
- 控制总价过滤（新增确认）：
  - 使用额外字段：`minBudget` / `maxBudget`（数字）
  - 5万以下：`maxBudget=50000`
  - 5-6万：`minBudget=50000` 且 `maxBudget=60000`
  - 6-7万：`minBudget=60000` 且 `maxBudget=70000`
  - 10万以上：`minBudget=100000`
  - 全部：`minBudget`/`maxBudget` 不传（不要传 null）

空值约定（新增确认）：
- `categoryType = null` 等价于“全部类目”，且该字段在“全部”场景需显式传 `null`。
- `minBudget`/`maxBudget` 在“全部”场景应省略（不要传 `null`）。

中间枚举约定（新增确认）：
- 项目内部使用无歧义枚举（例如 `Goods`/`Service`/`Engineering`），对外通过 serde 映射到 `GOODS`/`SERVICE`/`PROJECT`。
- 语义权威来源于页面中文筛选项（货物类/服务类/工程类）；`PROJECT` 仅是对方后端编码词，不作为我方语义命名依据。

产品封装建议（新增确认）：
- `stateList` 属于后端实现细节，GUI 不应对用户暴露原始状态码。
- GUI 仅暴露业务文案选项，内部完成文案 -> 状态码数组映射。
- 控制总价同样只暴露业务文案（5万以下/5-6万等），内部映射为 `minBudget/maxBudget`。

## 6.5 districtCodeList 风险项与实现规则（已收敛）

最新结论（基于多轮城市/地区组合实测）：
- `administrativeDistrictCodeList` 未观察到除空数组外的有效取值，当前固定传 `[]`。
- 地区筛选仅通过 `districtCodeList` 承载。
- `districtCodeList` 全选样本为数值升序代码集（161 项，无重复）。
- “不限制地区(空数组)”与“全选地区(完整代码集)”在结果上可能等价，但语义不同。
- 最新修正样本显示：在“江西省-南昌市”去掉东湖区后，`districtCodeList` 缺少 `360102/360100/360000`，其余（含 `980` 系列）保持。

MVP 实现规则（定版）：
1. 无地区筛选：`districtCodeList=[]`，`administrativeDistrictCodeList=[]`。
2. 单地区筛选：`districtCodeList=[code]`，`administrativeDistrictCodeList=[]`。
3. 多地区筛选：`districtCodeList=[code1,code2,...]`，`administrativeDistrictCodeList=[]`。
4. 全选地区：`districtCodeList` 传完整全选代码集，`administrativeDistrictCodeList=[]`。
5. 若返回结果疑似被 Cookie 地域限制，记录 Cookie 地域键名存在性并做一次“切换区划 Cookie 后重试”诊断。

历史排查结论：
- 已尝试多种地区组合，未出现需要传非空 `administrativeDistrictCodeList` 的场景。
- 全选地区代码集样本见：`docs/jxemall-district-codes-observed-full-selection.md`。
- 江西去掉东湖区样本见：`docs/jxemall-district-codes-observed-jx-minus-one-county.md`。
- `980` 系列层级已确认：`980000` -> `980700` -> (`980701` 或 `980799`)。
- 先前“去掉东湖区触发父级回收”的推断已撤销，待后续重新抓样本验证。
- 排序原理当前按“数值升序观测”处理，是否为后端强制约束待确认。

## 6.4 BIDDING 与 REVERSE 模块差异（新增确认）

共同点：
- 列表接口路由相同：`POST /api/sparta/announcement/listNewest`
- 请求头结构同构，最小头策略一致（`Content-Type/Cookie/Origin/Referer/X-Requested-With`）。

差异点：
- 模块切换主参数：`tradeModel`
  - 竞价采购：`BIDDING`
  - 反拍采购：`REVERSE`
- 默认“无筛选”样本差异：
  - 竞价样本常见为业务筛选态（例如 `categoryType=GOODS`、`stateList=[4]`、`instanceCodes` 多馆）。
  - 反拍样本当前确认：`categoryType=null`、`stateList=[3,4,5,6,7,10,12,50]`、`instanceCodes=["JXWSCS"]`。
- 归纳口径：除 `instanceCodes` 外，反拍与竞价筛选行为同构。
- `instanceCodes` 编码集合本身固定；REVERSE 当前固定取值 `JXWSCS`。
- 页面 Referer 路径：
  - 竞价采购：`/luban/bidding/newest?...`
  - 反拍采购：`/luban/reverse/newest?tradeModel=REVERSE&tradeStyle=REVERSE...`
- `type` 与 `categoryType`：
  - `categoryType` 仍表示业务类目（`GOODS/SERVICE/PROJECT/null`）。
  - `type` 在当前认知中是公告类型语义（响应/详情参数），不等价于 `tradeModel`。

实现约束（MVP）：
- listNewest 请求构造时，模块维度优先看 `tradeModel`，不要用 `type` 代替。
- storage 层保留 `tradeModel` 与 `type` 双字段，避免跨模块同 `requisitionId` 时语义丢失。
- 去重分流维度可扩展为：`source + requisitionId + tradeModel + type`（当字段存在时）。

## 6.2 会话与 Cookie 要求（v1 已确认）

当前接口请求依赖 Cookie 会话（已确认）。

观测到的关键 Cookie 键名（脱敏存档，不保存具体值）：
- SESSION
- uid / prod-jiangxi_uid
- platform_code / prod-jiangxi_platform_code
- tenant_code
- institution_id
- user_type
- wsid
- districtCode / districtName / districtType
- _zcy_log_client_uuid

实现约束：
- crawler 不应硬编码 Cookie 值。
- 运行时 Cookie 应来自用户登录态（浏览器会话或 Keychain 引用）。
- 文档与日志中仅记录键名与是否存在，禁止落明文 SESSION。

## 6.3 请求头规范（v1 已确认）

基于实测请求样本，`listNewest` 的请求头可分为三类：

必须项（建议强校验）：
- `Content-Type: application/json;charset=UTF-8`
- `Cookie: ...`（需包含登录会话与租户相关键）
- `Origin: https://www.jxemall.com`
- `Referer: https://www.jxemall.com/luban/bidding/newest?...`

建议项（提高兼容性）：
- `Accept: application/json, text/plain, */*`
- `X-Requested-With: XMLHttpRequest`
- `User-Agent: <browser ua>`

浏览器噪声项（MVP 可不强依赖）：
- `Sec-Fetch-*`
- `sec-ch-ua*`
- `Accept-Language`
- `Accept-Encoding`
- `Connection`

Cookie 关键键名（脱敏）：
- `_zcy_log_client_uuid`
- `districtCode` / `districtName` / `districtType`
- `platform_code` / `prod-jiangxi_platform_code`
- `wsid`
- `uid` / `prod-jiangxi_uid`
- `user_type`
- `tenant_code`
- `institution_id`
- `SESSION`

REVERSE 样本补充（已确认）：
- `Referer` 可为 `https://www.jxemall.com/luban/reverse/newest?...`
- 其余头部与 BIDDING 基本同构，MVP 仍采用同一最小请求头策略。

实现建议：
- 头部生成策略采用“最小必需 + 可选增强”，避免过度绑定浏览器环境。
- `Host`、`Content-Length` 由 HTTP 客户端自动生成，不在业务层手动设置。
- 若出现 4xx/反爬拦截，再按需补齐 `Sec-Fetch-*` 与 `sec-ch-ua*`。

## 7. 质量与异常处理

- `cityName/areaName` 可能为空：前端展示需允许缺省。
- `announcementId` 可能为 null：不得作为主键。
- `remainingMilliseconds` 为动态值：不作为持久化业务主字段，可选存储。
- 当 `success=false` 或 HTTP 异常：记录 `CRAWLER_SITE_CHANGED` / `COMMON_INTERNAL` 并触发重试策略。

## 8. 与检索相关的字段权重（MVP 定版默认）

MVP 默认权重（可配置）：
- title: 3.0
- orgName: 1.5
- categoryTypeText: 1.2
- districtName/provinceName/cityName: 1.0
- 其余拼接文本: 0.8

## 9. 待确认项（后续 crawler 实施阶段）

- listNewest 请求头与鉴权剩余要素（是否存在签名参数、token/header 额外校验）。
- listNewest 参数枚举全集（categoryType/stateList/instanceCodes 的全量可取值）。
- `tradeStyle` / `displayTradeStyle` 的精确定义仍待确认（仅在后续确有展示需求时再研究）。
- `type` 与 `categoryType` 的约束关系。
- `announcementType` 与 `relateAnnouncementTypes[].announcementTypeCode` 的稳定映射关系。

当前工作结论：
- `tradeModel` 不是单纯渲染字段，而是同一接口下的隐藏筛选项/模块切换参数。
- 返回值中的 trade 系列字段当前可视为与请求参数或模块上下文一一对应的镜像信息，不纳入独立业务语义建模。
- `type` 在竞价样本中暂未体现区分度（当前多为 `BIDDING_INVITATION`），但跨模块场景需保留并参与详情路由与分流。
- MVP 阶段仅在请求模型中保留 `tradeModel`；返回值中的 `tradeModel` / `tradeStyle` / `displayTradeStyle` 仅保留于原始响应层，不进入持久化业务模型与检索排序依据。
