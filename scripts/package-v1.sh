#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$ROOT_DIR/dist/v1"
TIMESTAMP="$(date +%Y%m%d-%H%M%S)"
LOG_FILE="$OUT_DIR/build-$TIMESTAMP.log"
MIN_FREE_GB="${PACKAGE_V1_MIN_FREE_GB:-2}"
MIN_FREE_KB=$((MIN_FREE_GB * 1024 * 1024))

mkdir -p "$OUT_DIR"

echo "[package-v1] workspace: $ROOT_DIR"
echo "[package-v1] output: $OUT_DIR"

FREE_KB="$(df -Pk "$ROOT_DIR" | awk 'NR==2 {print $4}')"
if [[ -z "$FREE_KB" ]]; then
  echo "[package-v1] ERROR: failed to detect free disk space" >&2
  exit 2
fi

if [[ "${PACKAGE_V1_SKIP_DISK_CHECK:-0}" != "1" ]] && (( FREE_KB < MIN_FREE_KB )); then
  FREE_GB="$(awk "BEGIN {printf \"%.2f\", $FREE_KB/1024/1024}")"
  NEED_GB="$(awk "BEGIN {printf \"%.2f\", $MIN_FREE_KB/1024/1024}")"
  cat <<MSG
[package-v1] ERROR: insufficient disk space for tauri build.

available: ${FREE_GB} GiB
required : >= ${NEED_GB} GiB

建议先释放空间后再打包，可优先清理：
1) 项目 target 目录
2) 下载目录和回收站
3) 其他大体积缓存

可选：
- 临时跳过检查：PACKAGE_V1_SKIP_DISK_CHECK=1 ./scripts/package-v1.sh
- 自定义阈值：PACKAGE_V1_MIN_FREE_GB=1 ./scripts/package-v1.sh
MSG
  exit 2
fi

if [[ ! -f "$ROOT_DIR/src-tauri/tauri.conf.json" && ! -f "$ROOT_DIR/src-tauri/tauri.conf.json5" ]]; then
  cat <<'MSG'
[package-v1] ERROR: src-tauri/tauri.conf.json not found.

当前仓库还未初始化 Tauri 工程。请先在项目根目录执行：
1) cargo install tauri-cli
2) cargo tauri init --ci

完成后重跑：
  ./scripts/package-v1.sh
MSG
  exit 2
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "[package-v1] ERROR: cargo not found" >&2
  exit 2
fi

if ! cargo tauri --version >/dev/null 2>&1; then
  cat <<'MSG'
[package-v1] ERROR: cargo tauri is not available.

请先安装：
  cargo install tauri-cli
MSG
  exit 2
fi

if ! : > "$LOG_FILE"; then
  echo "[package-v1] ERROR: cannot write build log file at $LOG_FILE" >&2
  echo "[package-v1] please free disk space first" >&2
  exit 2
fi

pushd "$ROOT_DIR/src-tauri" >/dev/null

echo "[package-v1] running cargo tauri build ..."
(
  set -x
  cargo tauri build
) 2>&1 | tee "$LOG_FILE"

popd >/dev/null

BUNDLE_DIR="$ROOT_DIR/target/release/bundle"
ARCHIVE_PATH="$OUT_DIR/tauri-v1-$TIMESTAMP.tar.gz"

if [[ -d "$BUNDLE_DIR" ]]; then
  tar -czf "$ARCHIVE_PATH" -C "$BUNDLE_DIR" .
  echo "[package-v1] packaged bundle archive: $ARCHIVE_PATH"
else
  echo "[package-v1] WARNING: bundle directory not found at $BUNDLE_DIR"
  echo "[package-v1] check build log: $LOG_FILE"
  exit 3
fi

echo "[package-v1] done"
