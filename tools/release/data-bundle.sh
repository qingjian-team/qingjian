#!/usr/bin/env bash
# 把产品词库与两份含章模型装进单个压缩包，发布为不可变的 data-vN Release，并写 tools/release/data.lock。
# CI 与自编译按锁文件取数据（data-fetch.sh）；改了数据发新号，锁文件与用到新数据的代码同一个提交。
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

PRODUCT_FILES=(dict.qj lm.qj glossary-en.qj glossary-ja.qj glossary-zh.qj glossary-es.qj glossary-vi.qj glossary-de.qj english.tsv english-frequency.tsv)
MODEL_FILE=data/models/hanzhang-zhiwei/hanzhang-zhiwei-small.qjm
P2C_MODEL_FILE=data/models/hanzhang-tongbian/hanzhang-tongbian-small.qjm

for f in "${PRODUCT_FILES[@]}"; do
  [[ -f "data/generated/$f" ]] || { echo "缺少 data/generated/$f，先按 assets/lexicon/QINGJIAN.md 生成" >&2; exit 1; }
done
DOMAIN_FILES=()
for f in data/generated/dicts/*.qj; do [[ -f "$f" ]] && DOMAIN_FILES+=("dicts/$(basename "$f")"); done
[[ ${#DOMAIN_FILES[@]} -gt 0 ]] || { echo "缺少 data/generated/dicts/*.qj（领域词库）" >&2; exit 1; }
# 随包辅码码表（笔画，issue #8）：由 tools/dict-convert 的 stroke + pack codes 生成，来源与许可见 assets/stroke/README.md
CODE_FILES=()
for f in data/generated/codes/*.qj; do [[ -f "$f" ]] && CODE_FILES+=("codes/$(basename "$f")"); done
if [ "$(ls -1 data/generated/codes/*.qj 2>/dev/null | wc -l)" -eq 0 ]; then
  echo "缺少 data/generated/codes/*.qj（随包笔画码表）：先跑 data/cns 的 stroke 与 pack codes，见 assets/stroke/README.md" >&2
  exit 1
fi
# 三件套比 .qjm 新（重训了没重打）就重打；没有三件套也没有 .qjm 就停。
[[ -f data/models/hanzhang-zhiwei/model.safetensors || -f "$MODEL_FILE" ]] || { echo "缺少 $MODEL_FILE，先把导出的三件套放到 data/models/hanzhang-zhiwei/ 再跑 tools/release/pack-model.sh" >&2; exit 1; }
[[ -f data/models/hanzhang-zhiwei/model.safetensors ]] && tools/release/pack-model.sh
[[ -f data/models/hanzhang-tongbian/model.safetensors || -f "$P2C_MODEL_FILE" ]] || { echo "缺少 $P2C_MODEL_FILE，先把导出的三件套放到 data/models/hanzhang-tongbian/ 再运行 tools/release/pack-model.sh" >&2; exit 1; }
[[ -f data/models/hanzhang-tongbian/model.safetensors ]] && QINGJIAN_MODEL_DIR=data/models/hanzhang-tongbian tools/release/pack-model.sh

rm -rf "$OUT" && mkdir -p "$OUT/stage/data/generated/dicts" "$OUT/stage/data/generated/codes" \
  "$OUT/stage/data/models/hanzhang-zhiwei" "$OUT/stage/data/models/hanzhang-tongbian"
for f in "${PRODUCT_FILES[@]}" "${DOMAIN_FILES[@]}" "${CODE_FILES[@]}"; do
  cp "data/generated/$f" "$OUT/stage/data/generated/$f"
done
cp "$MODEL_FILE" "$OUT/stage/$MODEL_FILE"
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

sha_of() { grep " ./$1\$" "$OUT/SHA256SUMS" | cut -d' ' -f1; }
gh release create "$TAG" --prerelease --target "$TARGET" --title "产品数据 $TAG" \
  --notes "单个 qingjian-data.tar.gz 包含运行时词库、语言模型、释义表、含章·通变（hanzhang-tongbian-small.qjm）和含章·知微（hanzhang-zhiwei-small.qjm）。仓库 tools/release/data.lock 钉住压缩包的 SHA-256。" \
  "$OUT/qingjian-data.tar.gz"

cat > "$LOCK" <<EOF
# 产品数据版本，data-bundle.sh 写、data-fetch.sh 读；不要手改
tag = $TAG
qingjian-data.tar.gz = $(sha_of qingjian-data.tar.gz)
EOF
echo "已发 ${TAG}，锁文件已更新（记得提交）"
