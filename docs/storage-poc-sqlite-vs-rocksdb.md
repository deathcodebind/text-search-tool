# 存储选型 PoC 对比与结论（SQLite vs RocksDB）

## 1. 背景

MVP 需要本地嵌入式存储来承载：
- 抓取记录元数据
- 规则配置
- 任务状态
- 3 天周期自动过期清理
- 与 Tantivy 索引联动删除

## 2. 核心结论

结论：MVP 阶段选择 SQLite 作为唯一元数据存储。

原因：
- 过期清理并非 SQLite 短板。对 expires_at 建索引后，定时 DELETE + 批处理即可稳定实现。
- 你当前业务需要多维筛选与状态查询，关系型查询能力在 SQLite 中更直接。
- 跨平台（Windows + macOS）与打包集成成熟，维护成本低。
- RocksDB 在高写入 KV 场景有优势，但会显著增加查询建模和维护复杂度。

## 3. 对比矩阵

| 维度 | SQLite | RocksDB |
|---|---|---|
| 过期清理实现 | 强。按 expires_at 索引 + 批量删除 | 中。需自实现 key 设计或依赖 compaction 行为 |
| 复杂查询（规则/任务/筛选） | 强。SQL 天然支持 | 弱。需二级索引或额外数据结构 |
| 一致性事务 | 强。事务语义直接可用 | 中。需应用层协调多键一致性 |
| 开发与调试成本 | 低 | 中到高 |
| 跨平台集成 | 强 | 强 |
| 读写吞吐上限 | 中（MVP 足够） | 高 |
| 维护复杂度 | 低 | 高 |

## 4. 过期清理设计（SQLite）

### 4.1 表结构关键字段
- records.expires_at（UTC 时间）
- records.expired（可选软删除标记）
- crawl_jobs.finished_at

### 4.2 索引建议

```sql
CREATE INDEX IF NOT EXISTS idx_records_expires_at ON records(expires_at);
CREATE INDEX IF NOT EXISTS idx_records_source_id ON records(source, source_record_id);
CREATE INDEX IF NOT EXISTS idx_records_source_url ON records(source, source_url);
```

### 4.3 清理流程

1. 任务调度器每天固定时刻运行。
2. 分批查询到期记录 ID（例如每批 1000）。
3. 事务内删除 metadata，并记录待删除索引 ID。
4. 提交后调用 search-engine 删除 Tantivy 文档。
5. 记录清理统计与耗时。

示例 SQL（硬删除）：

```sql
DELETE FROM records
WHERE expires_at <= ?
LIMIT 1000;
```

说明：若当前 SQLite 版本不支持 DELETE LIMIT，可先 SELECT id LIMIT N 再 DELETE WHERE id IN (...)

## 5. PoC 目标与验收

### 5.1 PoC 输入规模（建议）
- 10 万条 records
- 3 天窗口内持续增量写入
- 每日一次过期清理

### 5.2 验收指标
- 单次清理任务可在可接受时间内完成（例如 < 5s，视机器而定）
- 清理期间查询可用，无明显阻塞
- 清理后索引与数据库数据量一致
- 重复抓取去重结果正确

## 6. 风险与对策

风险 1：大批量删除导致数据库膨胀
- 对策：启用 WAL；定期执行 PRAGMA optimize；在低峰期执行 VACUUM（按需）

风险 2：清理与查询并发冲突
- 对策：小批量删除 + 间隔提交；设置合理 busy_timeout

风险 3：索引与数据库不一致
- 对策：采用“删除记录 ID 事件表”或失败重试队列，确保最终一致

## 7. 何时再评估 RocksDB

当出现以下信号时再评估迁移：
- 单机数据量显著超出 SQLite 舒适区（例如千万级持续增长）
- 写入吞吐成为主要瓶颈且 SQL 查询需求下降
- 可接受引入二级索引与更高维护复杂度

## 8. 最终决策

- MVP：SQLite + Tantivy
- 过期清理：基于 expires_at 索引的定时批处理删除
- 迁移策略：保留 storage 抽象层，后续可平滑接入 RocksDB
