# V1 客户预览包

## 一键打包

```bash
./scripts/package-v1.sh
```

脚本会做以下事情：

1. 检查是否存在 `src-tauri/tauri.conf.json`（未初始化会直接失败并给出指引）。
2. 检查 `cargo tauri` 是否可用。
3. 检查磁盘剩余空间阈值（默认 2 GiB，可配置）。
4. 执行 `cargo tauri build`。
5. 自动归档产物到 `dist/v1/tauri-v1-<timestamp>.tar.gz`。
6. 输出构建日志到 `dist/v1/build-<timestamp>.log`。

## V1 客户可演示功能

1. 登录流程：输入用户名 + 明文密码，客户端自动加密后走真实登录请求链路。
2. 拉取流程：按地区编码和关键词提示启动拉取任务，并可查询任务状态。
3. 多组关键词：通过表单配置组间 Bool 关系（`must` / `should` / `mustNot` + 组内最少命中）实时预览命中。

## 首次使用（仓库还没初始化 Tauri）

在项目根目录执行：

```bash
cargo install tauri-cli
cargo tauri init --ci
./scripts/package-v1.sh
```

若需临时调整磁盘预检：

```bash
PACKAGE_V1_SKIP_DISK_CHECK=1 ./scripts/package-v1.sh
# 或
PACKAGE_V1_MIN_FREE_GB=1 ./scripts/package-v1.sh
```

## 交付客户建议

- 交付归档包：`dist/v1/tauri-v1-<timestamp>.tar.gz`
- 同步交付构建日志：`dist/v1/build-<timestamp>.log`
- 附带本说明文档：`docs/release-v1.md`
