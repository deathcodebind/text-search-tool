# 全文搜索竞标助手 Tauri Command 与错误码映射（MVP）

## 1. 目标

为前端 Pinia action 提供稳定的命令调用协议与错误处理策略，确保：
- 每个 command 的成功返回可预测
- 每个错误码对应明确的 UI 行为
- 错误文案可本地化

## 2. 命令清单

### 2.1 认证与会话
- save_credential
- validate_session

### 2.2 抓取任务
- start_crawl
- get_crawl_job
- list_crawl_jobs

### 2.3 规则管理
- create_rule
- update_rule
- delete_rule
- list_rules
- toggle_rule

### 2.4 检索
- search_records
- get_record_detail

## 3. 错误响应结构

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

字段说明：
- code：机器可读错误码，前端据此分支处理。
- message：用户可读文案（可直接展示或经 i18n key 转换）。
- details：调试信息，不直接暴露敏感数据。

## 4. 命令级错误码映射

### 4.1 save_credential
可能错误码：
- COMMON_INVALID_ARGUMENT
- COMMON_INTERNAL

前端处理：
- 参数错误：表单项高亮 + 就地提示。
- 内部错误：toast + 引导查看日志。

### 4.2 validate_session
可能错误码：
- CRAWLER_AUTH_EXPIRED
- CRAWLER_SITE_CHANGED
- COMMON_INTERNAL

前端处理：
- AUTH_EXPIRED：跳转登录配置页并提示重新登录。
- SITE_CHANGED：提示站点变更，建议稍后重试并上报。

### 4.3 start_crawl
可能错误码：
- CRAWLER_AUTH_EXPIRED
- CRAWLER_RATE_LIMITED
- COMMON_INVALID_ARGUMENT
- COMMON_INTERNAL

前端处理：
- AUTH_EXPIRED：阻断任务启动，要求先验证会话。
- RATE_LIMITED：提示稍后再试，允许一键重试。
- INVALID_ARGUMENT：回填并高亮错误筛选参数。

### 4.4 get_crawl_job / list_crawl_jobs
可能错误码：
- COMMON_NOT_FOUND
- COMMON_INTERNAL

前端处理：
- NOT_FOUND：任务可能已清理，提示刷新列表。

### 4.5 create_rule / update_rule
可能错误码：
- SEARCH_QUERY_INVALID
- COMMON_INVALID_ARGUMENT
- STORAGE_WRITE_FAILED

前端处理：
- QUERY_INVALID：定位到规则编辑器并显示语义错误。
- WRITE_FAILED：提示保存失败并保留当前编辑内容。

### 4.6 delete_rule / toggle_rule
可能错误码：
- COMMON_NOT_FOUND
- STORAGE_WRITE_FAILED

前端处理：
- NOT_FOUND：提示规则已不存在并刷新列表。

### 4.7 search_records
可能错误码：
- SEARCH_QUERY_INVALID
- SEARCH_INDEX_UNAVAILABLE
- COMMON_INTERNAL

前端处理：
- QUERY_INVALID：显示规则错误并停止查询。
- INDEX_UNAVAILABLE：提示索引不可用并给出“重建索引”入口。

### 4.8 get_record_detail
可能错误码：
- COMMON_NOT_FOUND
- COMMON_INTERNAL

前端处理：
- NOT_FOUND：提示记录已过期或被删除，返回列表页。

## 5. Pinia action 处理规范

建议每个 action 返回统一结构：

```ts
export type ActionResult<T> =
  | { ok: true; data: T }
  | { ok: false; error: { code: string; message: string; details?: unknown } };
```

约定：
- action 内部不抛原始异常，统一转换为 ActionResult。
- 组件层只根据 ok/code 决定 UI 行为。

## 6. UI 行为分级

- 阻断级错误：无法继续流程（如 AUTH_EXPIRED、INDEX_UNAVAILABLE）。
- 可恢复错误：可重试或调整参数（如 RATE_LIMITED、QUERY_INVALID）。
- 提示级错误：不阻断主流程（如 NOT_FOUND 在列表刷新后可继续）。

## 7. 观测与日志

每次 command 失败记录：
- command_name
- code
- timestamp
- request_id
- source（可选）

避免记录：
- 明文账号密码
- 完整敏感 token

## 8. 建议默认文案

- CRAWLER_AUTH_EXPIRED：登录状态已失效，请重新登录后重试。
- CRAWLER_RATE_LIMITED：请求过于频繁，请稍后再试。
- CRAWLER_SITE_CHANGED：目标站点结构已变更，正在适配中。
- SEARCH_QUERY_INVALID：查询规则存在语义错误，请检查关键词条件。
- SEARCH_INDEX_UNAVAILABLE：索引暂不可用，请重建索引后重试。
- COMMON_INTERNAL：系统内部错误，请稍后重试。

## 9. 首批落地清单

1. 在 Rust 侧统一 AppError -> 前端 JSON 错误响应转换。
2. 在 Pinia 侧实现 normalizeError 工具函数。
3. 为 start_crawl、search_records、create_rule 三个核心命令先补错误分支测试。
4. 在 GUI 增加“错误详情折叠区”用于调试模式展示 details。
