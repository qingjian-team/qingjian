#!/usr/bin/env bash
# 产品数据不在 git 里（data/ 整个 gitignore），CI 打包时从仓库的 `data` Release 下载。
# 这个脚本把本机 data/generated/ 里输入法要随包的文件打成 qingjian-data.tar.gz，
# 把本地整句模型 data/model/model.qjm（训练仓库 ../train 导出三件套，tools/release/pack-model.sh 打成一个文件）原样上传，
# 把 LLM 生成的中间产物（续跑用的 JSONL）打成 qingjian-llm-intermediates.tar.gz，上传（覆盖）到 `data` Release。
#
#   tools/release/data-bundle.sh           # 打包并上传
#   tools/release/data-bundle.sh --pack    # 只打包到 target/release-data/，不上传
#
# `data` 是一个滚动的预发布（prerelease）Release：预发布不会成为 GitHub 的 latest，
# 所以官网取 releases/latest/download/releases.json 时拿到的仍是最新的版本发布。
# 词库 / 语言模型 / 释义表重生成或模型重训之后重跑一次即可；每次覆盖，不留历史。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="$ROOT/target/release-data"
cd "$ROOT"

PRODUCT_FILES=(dict.qj lm.qj glossary-en.qj glossary-ja.qj glossary-zh.qj english.tsv english-frequency.tsv)
MODEL_FILE=data/model/model.qjm
LLM_FILES=(gloss-llm.jsonl gloss-en-llm.jsonl pinyin-llm.jsonl)

for f in "${PRODUCT_FILES[@]}"; do
  [[ -f "data/generated/$f" ]] || { echo "缺少 data/generated/$f，先按 assets/lexicon/QINGJIAN.md 生成" >&2; exit 1; }
done
DOMAIN_FILES=()
for f in data/generated/dicts/*.qj; do [[ -f "$f" ]] && DOMAIN_FILES+=("dicts/$(basename "$f")"); done
[[ ${#DOMAIN_FILES[@]} -gt 0 ]] || { echo "缺少 data/generated/dicts/*.qj（领域词库）" >&2; exit 1; }
# 三件套比 .qjm 新（重训了没重打）就重打；没有三件套也没有 .qjm 就停（没有模型的包不会重排）
[[ -f data/model/model.safetensors || -f "$MODEL_FILE" ]] || { echo "缺少 $MODEL_FILE，先在训练仓库导出三件套到 data/model/ 再跑 tools/release/pack-model.sh" >&2; exit 1; }
[[ -f data/model/model.safetensors ]] && tools/release/pack-model.sh

rm -rf "$OUT" && mkdir -p "$OUT"
# 路径相对 data/generated/，CI 解到 data/generated/ 就与本机一样
tar -czf "$OUT/qingjian-data.tar.gz" -C data/generated "${PRODUCT_FILES[@]}" "${DOMAIN_FILES[@]}"
# 模型单独一个文件：只重训模型时不用重传词库；CI 放到 data/model/，bundle.sh / qingjian.iss 见到就随包（.qjm 内部已是 fp16，不再压）
cp "$MODEL_FILE" "$OUT/model.qjm"
present=()
for f in "${LLM_FILES[@]}"; do [[ -f "data/generated/$f" ]] && present+=("$f"); done
if [[ ${#present[@]} -gt 0 ]]; then
  tar -czf "$OUT/qingjian-llm-intermediates.tar.gz" -C data/generated "${present[@]}"
fi
(cd "$OUT" && shasum -a 256 ./*.tar.gz ./model.qjm | tee SHA256SUMS)
du -h "$OUT"/*.tar.gz "$OUT/model.qjm"

[[ "${1:-}" == "--pack" ]] && exit 0

if ! gh release view data >/dev/null 2>&1; then
  gh release create data --prerelease --title "产品数据（CI 打包用）" \
    --notes "输入法随包的词库 / 语言模型 / 释义表（qingjian-data.tar.gz）、本地整句模型单文件（model.qjm）与 LLM 生成的续跑中间产物（qingjian-llm-intermediates.tar.gz）。滚动覆盖，不是软件版本。由 tools/release/data-bundle.sh 上传。"
else
  gh release edit data --notes "输入法随包的词库 / 语言模型 / 释义表（qingjian-data.tar.gz）、本地整句模型单文件（model.qjm）与 LLM 生成的续跑中间产物（qingjian-llm-intermediates.tar.gz）。滚动覆盖，不是软件版本。由 tools/release/data-bundle.sh 上传。"
fi
gh release upload data "$OUT"/*.tar.gz "$OUT/model.qjm" "$OUT/SHA256SUMS" --clobber
# 早先上传的三件套 tar 已被 model.qjm 取代
gh release delete-asset data qingjian-model.tar.gz --yes 2>/dev/null || true
echo "已上传到 Release: data"
