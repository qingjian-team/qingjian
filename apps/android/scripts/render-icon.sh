#!/usr/bin/env bash
# 应用图标前景层：assets/icon/logo.png 缩进自适应图标的安全区（108dp 画布中间 66dp），xxxhdpi 一档，生成物随仓库提交。
# 用法：apps/android/scripts/render-icon.sh（要 ImageMagick）
set -euo pipefail
cd "$(dirname "$0")/../../.."
magick assets/icon/logo.png -resize 268x268 -background none -gravity center -extent 432x432 \
  apps/android/app/src/main/res/drawable-xxxhdpi/ic_launcher_foreground.png
