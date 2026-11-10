#!/usr/bin/env bash
# 取产品数据：语言模型与含章·通变按 tools/release/models.lock 从模型仓库取（models-fetch.py），
# 随包词库与码表按 tools/release/dicts.lock 从词库仓库取（dicts-fetch.py）。Gitee 与 GitHub 同时请求、先回来的用。
#
#   tools/release/data-fetch.sh            # 下载 + 校验 + 解开
#   tools/release/data-fetch.sh --verify   # 不联网，只校验 target/release-data/ 里的缓存与解开的文件
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
PYTHON="${PYTHON:-python3}"

"$PYTHON" tools/release/models-fetch.py "$@"
"$PYTHON" tools/release/dicts-fetch.py "$@"

# build-info.json 记下随包的语言模型与含章·通变的摘要（upload-build.sh 读）
if [[ -n "${GITHUB_ENV:-}" ]]; then
  sha256() { if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1; }
  {
    echo "LM_SHA256=$(sha256 data/generated/lm.qj)"
    echo "P2C_MODEL_SHA256=$(sha256 data/models/hanzhang-tongbian/hanzhang-tongbian-small.qjm)"
  } >> "$GITHUB_ENV"
fi
