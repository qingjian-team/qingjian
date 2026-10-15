#!/usr/bin/env bash
# 把 1024px 的 logo.png 栅格化成 freedesktop hicolor 各档 PNG，直接等比缩放、不做视觉重设计。
# 生成物随仓库提交（hicolor/ 下八档），安装与 AUR 打包都不依赖 ImageMagick；只有改了源图才需要重跑。
#   assets/icon/linux/render-icons.sh        # 需要 magick（ImageMagick 7）
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
SRC="$ROOT/assets/icon/logo.png"
OUT="$ROOT/assets/icon/linux/hicolor"
# 16/22/24 是状态栏托盘实际会查的尺寸，32/48/64 给菜单与通知，128/256 给应用中心与高分屏。
for size in 16 22 24 32 48 64 128 256; do
  dir="$OUT/${size}x${size}/apps"
  mkdir -p "$dir"
  magick "$SRC" -resize "${size}x${size}" "$dir/qingjian.png"
done
