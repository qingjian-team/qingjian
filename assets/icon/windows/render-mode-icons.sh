#!/usr/bin/env bash
# 把任务栏中 / 英 / A 图标的 SVG 栅格化成四档 DPI 的 8 位 alpha 蒙版，DLL 用 include_bytes! 嵌入，运行时按主题填色。
#   assets/icon/windows/render-mode-icons.sh      # 需要 rsvg-convert 与 magick
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
OUT="$ROOT/apps/windows/tsf/resources/mode"
mkdir -p "$OUT"
for name in zh en caps; do
  for size in 16 20 24 32; do
    rsvg-convert -w "$size" -h "$size" "$ROOT/assets/icon/windows/mode-$name.svg" -o "$OUT/tmp.png"
    magick "$OUT/tmp.png" -alpha extract -depth 8 "gray:$OUT/$name-$size.alpha"
  done
done
rm -f "$OUT/tmp.png"
ls -l "$OUT"
