#!/usr/bin/env bash
# 把含章模型的三件套打成同目录的正式命名 .qjm。
# 含章·通变放 data/models/hanzhang-tongbian/（QINGJIAN_MODEL_DIR 指过去），元数据按字表里有没有 <sep> 自动选。
# 随包只带 .qjm（mac Resources/models/<模型标识>/、Windows {app}\data\models\<模型标识>\）；三件套留给开发直接加载。
# 元数据（名称 / 型号 / 许可 / 署名）只写在这里；脚本改动后两份模型都会重打。
#
#   tools/release/pack-model.sh            # 三件套或脚本比 .qjm 新（或没有 .qjm）才重打
#   tools/release/pack-model.sh --force    # 总是重打（改了元数据）
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
MODEL_DIR="${QINGJIAN_MODEL_DIR:-data/models/hanzhang-zhiwei}"
WEIGHTS="$MODEL_DIR/model.safetensors"

# P2C 的字表含 <sep>，据此确定正式标识，不依赖目录名。
kind="$(python3 -c 'import json, sys; print("p2c" if "<sep>" in json.load(open(sys.argv[1]))["tokens"] else "sentence")' "$MODEL_DIR/vocab.json")"
if [[ "$kind" == "p2c" ]]; then
  name="含章·通变（Hanzhang Tongbian）"
  model_id="hanzhang-tongbian"
  attribution="青简训练的带噪拼音转汉字模型；语料：中文维基百科（CC-BY-SA-3.0 / GFDL）、LCCC（MIT）、IndustryCorpus2（Apache-2.0）"
else
  name="含章·知微（Hanzhang Zhiwei）"
  model_id="hanzhang-zhiwei"
  attribution="青简训练的字级语言模型；语料：中文维基百科（CC-BY-SA-4.0）、LCCC（MIT）"
fi
OUT="$MODEL_DIR/$model_id-small.qjm"

[[ -f "$WEIGHTS" ]] || { echo "缺少 $WEIGHTS，先把导出的三件套放到 $MODEL_DIR" >&2; exit 1; }
if [[ "${1:-}" != "--force" && -f "$OUT" &&
      ! "$WEIGHTS" -nt "$OUT" &&
      ! "$MODEL_DIR/config.json" -nt "$OUT" &&
      ! "$MODEL_DIR/vocab.json" -nt "$OUT" &&
      ! "$ROOT/tools/release/pack-model.sh" -nt "$OUT" ]]; then
  echo "已是最新：$OUT"
  exit 0
fi

# 权重与代码同一许可（2026-09-12 定），署名写清训练语料
cargo run --release -q -p qingjian-dict-convert -- --out-dir "$MODEL_DIR" pack model --input "$MODEL_DIR" \
  --output "$OUT" \
  --name "$name" --license "GPL-3.0-or-later" \
  --attribution "$attribution" \
  --source "https://github.com/qingjian-team/qingjian" --data-version "$model_id-small"
ls -la "$OUT"
