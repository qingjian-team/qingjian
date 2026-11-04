#!/usr/bin/env bash
# 应用图标：assets/icon/macos/app.png 生成 .icns，输入法、设置程序、剪贴板共用。
#
#   apps/macos/scripts/make-icns.sh <输出.icns>
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
OUT="$1"
ICONSET="$(mktemp -d)/Qingjian.iconset"
mkdir -p "$ICONSET"
for size in 16 32 128 256 512; do
  sips -z $size $size "$ROOT/assets/icon/macos/app.png" --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
  double=$((size * 2))
  sips -z $double $double "$ROOT/assets/icon/macos/app.png" --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$OUT"
rm -rf "$(dirname "$ICONSET")"
