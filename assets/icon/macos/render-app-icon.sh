#!/usr/bin/env bash
# 把 macOS 应用图标的 SVG 栅格化成 1024px 的 app.png，bundle.sh 打包时再由它生成 .icns（输入法包与设置程序共用）。
# 图形与 Windows 的 app.svg 相同，只是按 macOS 图标网格摆：1024 画布里居中 824 的圆角方块；
# 不照这个网格画，macOS 26 会把图标塞进一块灰底圆角方块里。
#   assets/icon/macos/render-app-icon.sh      # 需要 rsvg-convert；生成物随仓库提交
set -euo pipefail
DIR="$(cd "$(dirname "$0")" && pwd)"
rsvg-convert -w 1024 -h 1024 "$DIR/app.svg" -o "$DIR/app.png"
