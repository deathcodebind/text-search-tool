# AI Agent 开发与打包指南

## 1. 目标与适用范围

本文档面向自动化/半自动化 AI Agent，约束其在本仓库内进行：

1. 代码开发与改动。
2. 编译与测试验证。
3. Tauri 客户预览包打包。

适用目录：仓库根目录 `text-search-tool/`。

## 2. 项目结构速览

```text
text-search-tool/
├── Cargo.toml                # Workspace（包含 crates/* 与 src-tauri）
├── crates/
│   ├── crawler/              # 列表/详情抓取
│   ├── gui/                  # 登录 API 客户端等
│   ├── search-engine/        # 检索逻辑
│   ├── shared/               # 共享模型与错误
│   └── storage/              # SQLite 持久化
├── src-tauri/                # Tauri 命令入口与桌面应用
├── ui/                       # 前端页面与交互脚本
├── scripts/package-v1.sh     # 一键打包脚本
└── dist/v1/                  # 打包输出目录
```

## 3. Agent 开发工作流（必须遵循）

1. 先阅读改动相关模块，再动手改代码。
2. 优先做最小改动，不重构无关代码。
3. 任何用户可见行为变化，都要同步更新前端文案或文档。
4. 改动后至少执行目标 crate 的 `cargo check` 与 `cargo test`。
5. 如改动跨模块（`src-tauri` + `storage` + `ui`），至少跑：

```bash
cargo check -p app -q
cargo test -p app -q
cargo test -p storage -q
```

## 4. 本地开发常用命令

```bash
# 整体检查
cargo check --workspace

# 全量测试
cargo test --workspace

# 仅验证桌面命令层
cargo check -p app -q
cargo test -p app -q

# 仅验证存储层
cargo test -p storage -q
```

## 5. 标准打包流程（V1）

在仓库根目录执行：

```bash
./scripts/package-v1.sh
```

若磁盘空间检查阻塞，可临时跳过：

```bash
PACKAGE_V1_SKIP_DISK_CHECK=1 ./scripts/package-v1.sh
```

或降低阈值（默认 2GiB）：

```bash
PACKAGE_V1_MIN_FREE_GB=1 ./scripts/package-v1.sh
```

脚本行为：

1. 校验 Tauri 配置与 `cargo tauri` 可用性。
2. 校验磁盘空间阈值。
3. 执行 `cargo tauri build`。
4. 归档产物到 `dist/v1/tauri-v1-<timestamp>.tar.gz`。
5. 保存日志到 `dist/v1/build-<timestamp>.log`。

## 6. 产物与交付

1. 安装包归档：`dist/v1/tauri-v1-<timestamp>.tar.gz`
2. 构建日志：`dist/v1/build-<timestamp>.log`

建议将两者一起交付，方便异地系统测试时定位问题。

## 7. 常见失败与处理

1. `src-tauri/tauri.conf.json` 缺失：先执行 `cargo tauri init --ci`。
2. `cargo tauri` 不可用：先执行 `cargo install tauri-cli`。
3. 磁盘不足导致打包失败：清理空间或设置 `PACKAGE_V1_SKIP_DISK_CHECK=1`。
4. 某些详情任务长期停在 `queued/running`：列表接口会在超阈值后降级展示为 `timeout`，可用“重新拉取详情”单条重试。

## 8. 跨系统测试最小验收清单

1. 登录成功后可启动拉取。
2. 拉取列表可分页，详情状态可见。
3. 详情状态在 `queued/running/succeeded/failed/timeout` 之间正确流转。
4. `failed/timeout` 记录可点击“重新拉取详情”。
5. 重试次数展示为“不含首次尝试”。
