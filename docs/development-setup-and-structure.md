# 开发工作区结构与初始化（MVP）

## 1. 当前项目结构

```text
text-search-tool/
├── Cargo.toml
├── regions_simplified.json
├── docs/
└── crates/
    ├── crawler/
    ├── gui/
    ├── search-engine/
    ├── shared/
    └── storage/
```

## 2. Workspace 配置

根目录 `Cargo.toml`：

```toml
[workspace]
resolver = "3"
members = [
    "crates/*"
]
```

说明：
- 使用 `resolver = "3"` 以匹配 Rust 2024 edition 默认行为。
- 所有业务 crate 统一放在 `crates/` 下，由 workspace 统一管理。

## 3. 初始化命令（推荐复用）

```bash
mkdir -p crates
cargo new crates/storage --lib --vcs none
cargo new crates/search-engine --lib --vcs none
cargo new crates/crawler --lib --vcs none
cargo new crates/gui --lib --vcs none
cargo new crates/shared --lib --vcs none
```

说明：
- 优先使用 `cargo new` 生成结构，避免手工创建文件导致模板不一致。
- 使用 `--vcs none` 避免在子 crate 下产生独立 git 仓库。

## 4. 各 crate 职责边界

- `storage`：本地持久化、去重、过期清理、任务状态。
- `search-engine`：索引、分词、规则解析、评分与检索。
- `crawler`：站点登录态维护、增量抓取、字段标准化。
- `gui`：Tauri + 前端交互层，承载用户主流程。
- `shared`：跨 crate 通用模型、错误定义、分页与配置。

## 5. 日常开发命令

```bash
# 检查整个工作区
cargo check --workspace

# 运行全部单元测试
cargo test --workspace

# 格式化代码
cargo fmt --all

# Clippy 静态检查
cargo clippy --workspace --all-targets -- -D warnings
```

## 6. 推荐开发顺序

1. `crawler` + `storage` 打通抓取与入库最小链路。
2. `search-engine` 接入最小检索链路（先跑通，再优化规则）。
3. `gui` 串联抓取与查询，形成端到端流程。
4. 基于 `jxemall-district-testcases-confirmed.md` 补齐关键测试。

## 7. 下一步建议（本周）

1. 在 `shared` 中先稳定基础 DTO（地区筛选、分页、错误码）。
2. 为地区规则先落地单元测试，再接入模块实现。
3. 为 `storage` 增加最小 SQLite schema 与去重测试。
