# jxemall 查询请求规则待补位清单（模板）

## 1. 目标

用于记录非原生前端条件下对请求规则的观察、假设与验证结果。
每条规则都应标注“来源证据”和“验证状态”，避免后续误判。

配套参考：
- 页面筛选项与 payload 映射表见 `docs/jxemall-filter-ui-mapping.md`。

## 2. 请求基础信息

- 接口路径：
- listNewest：`/api/sparta/announcement/listNewest`
- detail：`/api/sparta/announcement/detail`
- 方法（GET/POST）：
- listNewest：POST（已确认）
- detail：GET（已确认）
- Content-Type：application/json（已确认，listNewest）
- 是否携带 Cookie：是（已确认）
- 是否需要签名字段：当前未观测到（待持续验证）
- 是否有时间戳/nonce：当前未观测到（待持续验证）
- 是否有防重放参数：当前未观测到（待持续验证）

已确认关键请求头：
- `Content-Type: application/json;charset=UTF-8`
- `Origin: https://www.jxemall.com`
- `Referer: https://www.jxemall.com/luban/bidding/newest?...`
- `X-Requested-With: XMLHttpRequest`
- `Cookie: ...`（需包含登录态与租户键）

REVERSE 模块补充（已确认）：
- 同一路由：`POST /api/sparta/announcement/listNewest`
- Referer 可为：`https://www.jxemall.com/luban/reverse/newest?tradeModel=REVERSE&tradeStyle=REVERSE...`
- 请求头结构与 BIDDING 基本同构。

详情 API（已确认）：
- 路径：`/api/sparta/announcement/detail`
- 参数：`requisitionId`（string）、`type`、`timestamp`
- 返回内容用于详情页渲染（优先 API，不做 HTML 解析）
- 方法：GET（已确认）

## 3. 请求参数清单

| 参数名 | 位置(query/body/header) | 类型 | 是否必填 | 示例值 | 猜测含义 | 证据来源 | 验证状态 |
|---|---|---|---|---|---|---|---|
|  |  |  |  |  |  |  | 未验证 |

验证状态枚举：
- 未验证
- 已验证
- 证伪

## 4. 分页规则

- 分页参数名称：
- 页码起始（0/1）：
- pageSize 上限：
- total/pages 字段是否可信：
- 翻页终止条件：

## 5. 筛选规则

- 地区筛选参数：
- 类目筛选参数：
- 状态筛选参数（state；subState 当前跳过）：
- 时间区间筛选参数：
- 多选参数编码方式（逗号/数组/重复键）：

地区筛选专项（建议新增记录）：
- `districtCodeList` 承载地区筛选；`administrativeDistrictCodeList` 当前固定为空数组。
- 空数组等价于“不过滤地区”。
- “全选地区”时 `districtCodeList` 为完整代码集，不等价于空数组。
- Cookie 的 `districtCode/districtType` 是否会覆盖 body 地区筛选。
- 已验证结论：多轮组合测试未出现 `administrativeDistrictCodeList` 非空有效场景。
- 排序原理：全选代码集当前观测为数值升序（161 项、无重复），是否为协议硬约束待确认。
- 样本补充（已修正）：去掉东湖区后，`districtCodeList` 缺少 `360102/360100/360000` 三个行政层级码，`980` 系列保持不变。
- 已确认映射：`980000`(培训省)、`980700`(江西培训省)、`980701`(江西培训市)、`980799`(江西培训省本级)。
- 南昌扩展节点映射（已确认名称）：`360191`(南昌高新技术产业开发区)、`360192`(南昌经济技术开发区)、`360193`(红谷滩区)、`360194`(南昌临空经济区)、`360199`(南昌市本级)。
- 景德镇扩展节点映射（已确认名称）：`360291`(景德镇高新技术产业开发区)、`360292`(江西景德镇陶瓷工业园区)、`360293`(景德镇昌南新区)、`360299`(景德镇市本级)。
- 萍乡扩展节点映射（已确认名称）：`360391`(萍乡经济技术开发区)、`360399`(萍乡市本级)。
- 经验规律（待继续验证）：当前已确认样本中，扩展码末位 `99` 常对应“市本级”；实现侧不应仅凭码位模式自动命名。
- 九江扩展节点映射（已确认名称）：`360491`(九江经济技术开发区(出口加工区))、`360492`(江西省庐山风景名胜区管理局)、`360493`(庐山西海风景名胜区)、`360494`(九江市八里湖新区)、`360499`(九江市本级)。
- 新余扩展节点映射（已确认名称）：`360591`(新余市仙女湖风景名胜区)、`360592`(新余高新技术产业开发区)、`360599`(新余市本级)。
- 鹰潭扩展节点映射（已确认名称）：`360622`(余江区，平台码)、`360691`(鹰潭高新技术产业开发区)、`360692`(信江新区)、`360693`(鹰潭市龙虎山风景名胜区)、`360699`(鹰潭市本级)。
- 码表差异判定（当前规则）：`360622` 与本地行政码口径不一致，优先判定为平台码表滞后或纰漏，而非“地区缺失”。实现上采用“对外兼容平台码、对内保留标准行政区语义”的双轨映射。
- 赣州扩展节点映射（已确认名称）：`360791`(赣州经济技术开发区)、`360792`(赣州蓉江新区)、`360793`(江西龙南经济技术开发区)、`360799`(赣州市本级)。
- 吉安扩展节点映射（已确认名称）：`360891`(井冈山经济技术开发区)、`360892`(吉安市庐陵新区)、`360899`(吉安市本级)。
- 宜春扩展节点映射（已确认名称）：`360990`(宜春经济技术开发区)、`360991`(宜春市宜阳新区)、`360992`(宜春市明月山温泉风景名胜区)、`360999`(宜春市本级)。
- 抚州扩展节点映射（已确认名称）：`361091`(抚州高新技术产业开发区)、`361092`(抚州市东临新区)、`361099`(抚州市本级)。
- 上饶扩展节点映射（已确认名称）：`361191`(上饶经济技术开发区)、`361192`(江西省三清山风景名胜区)、`361193`(上饶高铁经济试验区)、`361199`(上饶市本级)。
- 赣江新区虚拟市级单位（已确认）：`361200`(赣江新区)、`361209`(中医药科创城)、`361299`(赣江新区本级)。
- 编码到名称映射文件：`docs/jxemall-district-code-name-map.json`（纯 `code -> name`，供程序直接读取，单点维护）。
- 市本级语义（当前规则）：可作为上一级城市节点的虚拟标记码，用于表示城市层面的筛选，不等价于普通区县节点。
- 项目层级语义（当前规则）：平台项目存在层级差异，多数项目落在区县层级，部分项目直接落在市级层级；后者通常通过“市本级”虚拟码承载。
- 省本级虚拟码（已确认）：`369900`（江西省本级）。判定依据：该码在全选代码集中作为唯一省级孤立扩展码出现（不在 `regions_simplified.json` 行政区树中），且与“市本级虚拟码”语义一致。

## 6. 排序规则

- 排序字段参数：
- 升降序参数：
- 默认排序：
- 同分二级排序：

## 7. 会话与鉴权

- 关键 Cookie 名称：SESSION、uid/prod-jiangxi_uid、platform_code/prod-jiangxi_platform_code、tenant_code、institution_id、user_type、wsid、districtCode/districtName/districtType、_zcy_log_client_uuid
- 请求头策略：必需头最小集优先，浏览器噪声头按需补充
- Token 存储位置：
- 过期表现（HTTP 码/业务码）：
- 刷新机制：

## 8. 异常码与重试策略

| 场景 | HTTP 状态 | 业务字段(code/message) | 建议错误码 | 重试策略 |
|---|---|---|---|---|
| 登录过期 |  |  | CRAWLER_AUTH_EXPIRED | 需重新登录 |
| 频率限制 |  |  | CRAWLER_RATE_LIMITED | 指数退避 |
| 结构变化 |  |  | CRAWLER_SITE_CHANGED | 停止并告警 |

## 9. 证据留存建议

每次补充规则时至少保留：
- 请求样本（去敏）
- 响应样本（去敏）
- 触发该规则的操作步骤
- 当时页面路径与时间

## 10. 当前已确认（来自 listNewest）

- 主来源 ID：requisitionId（用于详情关联与主去重）
- `requisitionId` 由服务端以字符串下发，入库/比较/传参全链路按字符串处理（禁止转整数）。
- 辅助 ID：biddingId（用于回退匹配与审计）
- 时间字段：pubTimestamp/startTimestamp/endTimestamp（ms）
- 列表字段存在大量 nullable，需按弱约束处理
- categoryType 在“全部”场景下为 null（按合法语义处理）
- `categoryType` 是类目维度（`GOODS/SERVICE/PROJECT/null`），与模块切换参数 `tradeModel` 不同维度。
- `type` 当前定位为公告类型字段（响应/详情参数），不是 listNewest 的主筛选入参。
- 空值实现策略：
- `categoryType` 在“全部”场景显式传 `null`。
- `minBudget`/`maxBudget` 在“全部”场景省略字段，不传 `null`。
- stateList 映射：全部=[3,4,5,6,7,10,12,50]，未开始=[3]，竞价中=[4]，已过期=[5,6,7,10,12,50]
- REVERSE 无筛选样本：`tradeModel=REVERSE`、`categoryType=null`、`stateList=[3,4,5,6,7,10,12,50]`、`instanceCodes=["JXWSCS"]`。
- 口径归纳：除 `instanceCodes` 外，REVERSE 与 BIDDING 其他筛选行为按同构处理。
- `instanceCodes` 编码集合固定（`JXWSCS/JXDDCG/JXXYGH/JXFWGC`），REVERSE 当前固定取值为 `JXWSCS`。
- 地区筛选当前定版为：`districtCodeList` 生效，`administrativeDistrictCodeList` 固定 `[]`。
- 全选地区代码集样本见：`docs/jxemall-district-codes-observed-full-selection.md`。
- 江西去掉东湖区样本见：`docs/jxemall-district-codes-observed-jx-minus-one-county.md`。
- 产品约束：不在用户界面暴露原始状态码，只展示业务文案
- `budget` / `dealAmount` 当前按“元”处理。
- `subState` 当前跳过，不纳入 MVP 模型与筛选逻辑。
- 当前已采集的竞价响应样本中，`type` 多为 `BIDDING_INVITATION`。
- 当前 REVERSE 列表请求体样本未观测到 `type` 入参。
- 详情 API 路径已确认：`/api/sparta/announcement/detail`。
- 详情页 URL 模板：`/luban/bidding/detail?requisitionId={requisitionId}&type={type}`（`utm` 忽略）。
- 详情响应外层结构：`success/result/code/message`。
- 详情正文字段：`result.content`（HTML）。
- 详情标题字段：`result.title`。
- 详情发布时间字段：`result.releasedAt`（`yyyy-MM-dd HH:mm:ss`）。
- 详情附件字段：`result.attachments[]`（包含 `attachmentName/size/url`）。
- 详情关联校验字段：`result.relateAnnouncementTypes[].requisitionId`（string）。

当前工作结论：
- `tradeModel` 是隐藏筛选项/模块切换参数，而非单纯展示字段。
- 当前已知 `tradeModel` 至少包含 `BIDDING` 与 `REVERSE` 两种取值。
- 跨模块抓取以 `tradeModel` 为主开关，不以 `type` 代替。
- 返回值中的 trade 系列字段当前视为请求参数/模块上下文的镜像信息，MVP 暂不做独立语义建模。
- 返回值中的 trade 系列字段不参与正文 `content` 拼接。
- `type` 在当前竞价样本中未体现区分度，暂不纳入 listNewest 核心筛选逻辑。
