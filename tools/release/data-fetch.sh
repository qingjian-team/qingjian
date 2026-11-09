#!/usr/bin/env bash
# 按 tools/release/data.lock 下载并校验单包产品数据；旧锁仍可读取分离的模型附件。
#
#   tools/release/data-fetch.sh            # 下载 + 校验 + 解开
#   tools/release/data-fetch.sh --verify   # 只校验 target/release-data/ 里已下载的文件
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
LOCK="$ROOT/tools/release/data.lock"
OUT="$ROOT/target/release-data"
REPO="qingjian-team/qingjian"
ASSETS=(qingjian-data.tar.gz)
cd "$ROOT"

[[ -f "$LOCK" ]] || { echo "缺少 $LOCK" >&2; exit 1; }
lock_value() { sed -nE "s/^$1 *= *//p" "$LOCK" | head -1 | tr -d '[:space:]'; }
sha256() { if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1; }

TAG="$(lock_value tag)"
[[ -n "$TAG" ]] || { echo "$LOCK 里没有 tag" >&2; exit 1; }
LEGACY_MODEL="$(lock_value model.qjm)"
if [[ -n "$LEGACY_MODEL" ]]; then
  ASSETS+=(model.qjm)
  [[ -n "$(lock_value model-p2c.qjm)" ]] && ASSETS+=(model-p2c.qjm)
fi
mkdir -p "$OUT"

if [[ "${1:-}" != "--verify" ]]; then
  for f in "${ASSETS[@]}"; do
    rm -f "$OUT/$f"
    if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
      gh release download "$TAG" --repo "$REPO" --pattern "$f" --dir "$OUT"
    else
      curl -fL --retry 3 -o "$OUT/$f" "https://github.com/$REPO/releases/download/$TAG/$f"
    fi
  done
fi

for f in "${ASSETS[@]}"; do
  expected="$(lock_value "$f")"
  actual="$(sha256 "$OUT/$f")"
  [[ -n "$expected" && "$actual" == "$expected" ]] || { echo "$f 与 data.lock 不符（$TAG）：期望 $expected，实际 $actual" >&2; exit 1; }
  echo "$f  $actual"
done
[[ "${1:-}" == "--verify" ]] && exit 0

if [[ -n "$LEGACY_MODEL" ]]; then
  mkdir -p data/generated data/models/hanzhang-zhiwei
  tar -xzf "$OUT/qingjian-data.tar.gz" -C data/generated
  cp "$OUT/model.qjm" data/models/hanzhang-zhiwei/model.qjm
  MODEL_SOURCE=data/models/hanzhang-zhiwei/model.qjm
  P2C_MODEL_SOURCE=""
  if [[ -n "$(lock_value model-p2c.qjm)" ]]; then
    mkdir -p data/models/hanzhang-tongbian
    cp "$OUT/model-p2c.qjm" data/models/hanzhang-tongbian/model.qjm
    P2C_MODEL_SOURCE=data/models/hanzhang-tongbian/model.qjm
  fi
else
  tar -xzf "$OUT/qingjian-data.tar.gz" -C "$ROOT"
  # data-v7 及以前的包里还带着含章·知微，之后的不带
  MODEL_SOURCE=""
  [[ -f data/models/hanzhang-zhiwei/hanzhang-zhiwei-small.qjm ]] && MODEL_SOURCE=data/models/hanzhang-zhiwei/hanzhang-zhiwei-small.qjm
  P2C_MODEL_SOURCE=data/models/hanzhang-tongbian/hanzhang-tongbian-small.qjm
  [[ -f "$P2C_MODEL_SOURCE" && -f data/generated/dict.qj ]] || {
    echo "产品数据包缺少词库或含章·通变（$TAG）" >&2; exit 1;
  }
fi
# 解出来的 mtime 比 checkout 出来的 TSV 旧，bundle.sh 会以为要重打
find data/generated -type f -exec touch {} +
[[ -n "$MODEL_SOURCE" ]] && touch "$MODEL_SOURCE"
[[ -n "$P2C_MODEL_SOURCE" ]] && touch "$P2C_MODEL_SOURCE"
echo "产品数据 $TAG 已就位"

if [[ -n "${GITHUB_ENV:-}" ]]; then
  {
    echo "DATA_TAG=$TAG"
    echo "DATA_SHA256=$(lock_value qingjian-data.tar.gz)"
    if [[ -n "$MODEL_SOURCE" ]]; then
      echo "MODEL_SHA256=$(sha256 "$MODEL_SOURCE")"
    else
      echo "MODEL_SHA256="
    fi
    if [[ -n "$P2C_MODEL_SOURCE" ]]; then
      echo "P2C_MODEL_SHA256=$(sha256 "$P2C_MODEL_SOURCE")"
    else
      echo "P2C_MODEL_SHA256="
    fi
  } >> "$GITHUB_ENV"
fi
