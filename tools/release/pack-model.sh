#!/usr/bin/env bash
# 把训练仓库导出到 data/model/ 的三件套（model.safetensors / config.json / vocab.json）打成一个 data/model/model.qjm。
# 随包只带这一个文件（mac Resources/model/、Windows {app}\data\model、data Release）；三件套留在目录里给开发直接加载。
# 元数据（名称 / 许可 / 署名）只写在这里，bundle.sh 与 data-bundle.sh 见三件套比 .qjm 新就调它重打。
#
#   tools/release/pack-model.sh            # 三件套比 .qjm 新（或没有 .qjm）才重打
#   tools/release/pack-model.sh --force    # 总是重打（改了元数据）
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
MODEL_DIR="${QINGJIAN_MODEL_DIR:-data/model}"
WEIGHTS="$MODEL_DIR/model.safetensors"
OUT="$MODEL_DIR/model.qjm"

[[ -f "$WEIGHTS" ]] || { echo "缺少 $WEIGHTS，先在训练仓库（../train）export.py 导出三件套到 $MODEL_DIR" >&2; exit 1; }
if [[ "${1:-}" != "--force" && -f "$OUT" && ! "$WEIGHTS" -nt "$OUT" ]]; then
  echo "已是最新：$OUT"
  exit 0
fi

# 训练步数与预设写进数据版本，日志里认得出是哪一版模型
step="$(python3 -c 'import json, sys; c = json.load(open(sys.argv[1])); print("%s-%s" % (c.get("preset", ""), c.get("step", "")))' "$MODEL_DIR/config.json")"
# 权重与代码同一许可（2026-09-12 定），署名写清训练语料
cargo run --release -q -p qingjian-dict-convert -- --out-dir "$MODEL_DIR" pack model --input "$MODEL_DIR" \
  --name "青简整句模型" --license "GPL-3.0-or-later" \
  --attribution "青简训练的字级语言模型；语料：中文维基百科（CC-BY-SA-4.0）、LCCC（MIT）" \
  --source "https://github.com/qingjian-team/qingjian" --data-version "$step"
ls -la "$OUT"
