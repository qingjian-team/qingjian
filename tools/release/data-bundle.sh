#!/usr/bin/env bash
# 把语言模型、笔画码表与含章·通变装进单个压缩包，发布为不可变的 data-vN Release，并写 tools/release/data.lock。
# CI 与自编译按锁文件取数据（data-fetch.sh）；改了数据发新号，锁文件与用到新数据的代码同一个提交。
# 词库、英文词表、释义表在词库仓库 qingjian-dicts 发版，不进这个包，随包哪几本见 tools/release/dicts.lock。
#
#   tools/release/data-bundle.sh                 # 发到下一个 data-vN
#   tools/release/data-bundle.sh --tag data-v7   # 指定标签；已存在就拒绝
#   tools/release/data-bundle.sh --tag data-v3 --target main # 本地发版提交未推送时，标签指向远端 main
#   tools/release/data-bundle.sh --pack          # 只打包到 target/release-data/
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="$ROOT/target/release-data"
LOCK="$ROOT/tools/release/data.lock"
cd "$ROOT"

MODE=upload
TAG=""
TARGET="$(git rev-parse HEAD)"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --pack) MODE=pack; shift ;;
    --tag) TAG="$2"; shift 2 ;;
    --target) TARGET="$2"; shift 2 ;;
    *) echo "未知参数 $1" >&2; exit 1 ;;
  esac
done

PRODUCT_FILES=(lm.qj)
P2C_MODEL_FILE=data/models/hanzhang-tongbian/hanzhang-tongbian-small.qjm

for f in "${PRODUCT_FILES[@]}"; do
  [[ -f "data/generated/$f" ]] || { echo "缺少 data/generated/$f" >&2; exit 1; }
done
# 随包辅码码表（笔画，issue #8）：由 tools/dict-convert 的 stroke + pack codes 生成，来源与许可见 assets/stroke/README.md
CODE_FILES=()
for f in data/generated/codes/*.qj; do [[ -f "$f" ]] && CODE_FILES+=("codes/$(basename "$f")"); done
if [ "$(ls -1 data/generated/codes/*.qj 2>/dev/null | wc -l)" -eq 0 ]; then
  echo "缺少 data/generated/codes/*.qj（随包笔画码表）：先跑 data/cns 的 stroke 与 pack codes，见 assets/stroke/README.md" >&2
  exit 1
fi
# 三件套比 .qjm 新（重训了没重打）就重打；没有三件套也没有 .qjm 就停。含章·知微不进数据包：有通变时三个桌面平台都不加载它。
[[ -f data/models/hanzhang-tongbian/model.safetensors || -f "$P2C_MODEL_FILE" ]] || { echo "缺少 $P2C_MODEL_FILE，先把导出的三件套放到 data/models/hanzhang-tongbian/ 再运行 tools/release/pack-model.sh" >&2; exit 1; }
[[ -f data/models/hanzhang-tongbian/model.safetensors ]] && QINGJIAN_MODEL_DIR=data/models/hanzhang-tongbian tools/release/pack-model.sh

rm -rf "$OUT/stage" "$OUT/qingjian-data.tar.gz" && mkdir -p "$OUT/stage/data/generated/codes" \
  "$OUT/stage/data/models/hanzhang-tongbian"
for f in "${PRODUCT_FILES[@]}" "${CODE_FILES[@]}"; do
  cp "data/generated/$f" "$OUT/stage/data/generated/$f"
done
cp "$P2C_MODEL_FILE" "$OUT/stage/$P2C_MODEL_FILE"
COPYFILE_DISABLE=1 tar -czf "$OUT/qingjian-data.tar.gz" -C "$OUT/stage" data
(cd "$OUT" && shasum -a 256 ./qingjian-data.tar.gz | tee SHA256SUMS)
du -h "$OUT/qingjian-data.tar.gz"

[[ "$MODE" == "pack" ]] && exit 0

if [[ -z "$TAG" ]]; then
  last="$(gh release list --limit 200 --json tagName --jq '.[].tagName' | grep -E '^data-v[0-9]+$' | sed 's/data-v//' | sort -n | tail -1)"
  TAG="data-v$(( ${last:-0} + 1 ))"
fi
[[ "$TAG" =~ ^data-v[0-9]+$ ]] || { echo "标签要写成 data-vN：$TAG" >&2; exit 1; }
gh release view "$TAG" >/dev/null 2>&1 && { echo "$TAG 已存在，数据版本不覆盖" >&2; exit 1; }

# Windows 的 Git Bash 里 shasum 按二进制模式在文件名前加 `*`（`哈希 *./文件`），两种写法都要认
sha_of() { grep -E " \*?\./$1\$" "$OUT/SHA256SUMS" | cut -d' ' -f1; }
gh release create "$TAG" --prerelease --target "$TARGET" --title "产品数据 $TAG" \
  --notes "单个 qingjian-data.tar.gz 包含语言模型、笔画码表、含章·通变（hanzhang-tongbian-small.qjm）；词库在 qingjian-dicts 仓库。仓库 tools/release/data.lock 钉住压缩包的 SHA-256。" \
  "$OUT/qingjian-data.tar.gz"

cat > "$LOCK" <<EOF
# 产品数据版本，data-bundle.sh 写、data-fetch.sh 读；不要手改
tag = $TAG
qingjian-data.tar.gz = $(sha_of qingjian-data.tar.gz)
EOF
echo "已发 ${TAG}，锁文件已更新（记得提交）"
