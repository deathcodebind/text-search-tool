# 全文搜索竞标助手 技术方案（MVP）

## 1. 总体架构

采用多 crates Rust 工程，Tauri 作为桌面容器，Tantivy 负责全文检索，嵌入式数据库负责元数据与任务状态。

## 2. Crates 划分

- crates/crawler：站点登录、抓取、字段映射、增量拉取。
- crates/storage：元数据存取、去重、过期清理、任务状态。
- crates/search-engine：索引构建、查询解析、评分计算、分词接入。
- crates/gui：Tauri + Vue3 + Pinia 前端交互。
- crates/shared（可选）：公共模型、错误码、配置定义。

## 3. 数据流

1. GUI 发起抓取任务。
2. crawler 根据时间窗口抓取增量数据。
3. storage 执行去重并持久化。
4. search-engine 增量更新 Tantivy 索引。
5. GUI 提交规则查询。
6. search-engine 执行 Bool Query 核心子集并返回结果。

### 3.1 jxemall 列表接口映射

- 已确认首个列表来源接口：`/api/sparta/announcement/listNewest`。
- 字段映射与增量策略详见：`docs/jxemall-listnewest-data-profile.md`。
- 关键主键口径：`source + requisitionId`（`type` 非空时追加分流键 `source + requisitionId + type`，回退辅键 `source + biddingId`）。
- 时间口径：`pubTimestamp/endTimestamp`（epoch ms）统一转换为 UTC 存储。

## 4. 查询模型

### 4.1 规则能力（MVP）
- must
- should
- must_not
- minimum_should_match
- keyword boost（强弱关键词）

### 4.2 评分模型（可配置）
Score = a * keyword_score + b * field_weight_score + c * freshness_score

- keyword_score：关键词命中及权重。
- field_weight_score：标题/正文等字段加权。
- freshness_score：基于发布时间的时间衰减。
- a/b/c：配置文件可调整。

## 5. 中文分词方案

- 首选 jieba-rs（可控、生态成熟）。
- 评估 tantivy-jieba 作为集成加速方案。
- 支持自定义词典与停用词（MVP 可做静态配置）。

## 6. 存储方案

### 6.1 索引存储
- Tantivy 索引目录本地持久化。

### 6.2 元数据存储
- 数据库选型：SQLite（MVP 确定）。
- 记录项目元数据、抓取任务、规则配置、过期状态。
- 主去重键：来源 ID。
- 辅助键：URL。

### 6.3 数据过期
- 3 天周期清理任务。
- 清理策略：基于 expires_at 索引分批删除，并同步更新索引。
- RocksDB 仅作为后续规模化备选，不纳入首发实现。

## 7. 凭证与安全

- 优先使用系统 Keychain 存储登录凭证。
- 会话 Token 使用最小权限与最短生命周期策略。
- 记录关键操作日志但避免明文敏感信息。

## 8. GUI 方案

- 框架：Vue3。
- 状态管理：Pinia（替代重度 useContext 方案）。
- 页面建议：
  - 数据源与登录配置
  - 抓取任务与状态
  - 规则管理
  - 检索结果与详情

## 9. 发布与平台

- MVP 首发平台为 Windows + macOS 双平台。

## 10. 测试策略（MVP）

- 单元测试：规则解析、评分逻辑、去重与过期算法。
- 集成测试：抓取 -> 入库 -> 索引 -> 检索闭环。
- 回归测试：关键用户流程 UI 自动化冒烟。
- 性能测试：延后到主要功能稳定后执行，目标查询延迟 300ms 以内。

## 11. 风险与应对

- 站点结构变化：适配器化抽象 + 快速修复机制。
- 登录风控：手动介入与失败重试策略。
- 分词精度波动：行业词典可更新机制。
