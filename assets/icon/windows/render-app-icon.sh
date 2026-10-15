#!/usr/bin/env bash
# 把应用图标的 SVG 栅格化成多档 PNG 合成 qingjian.ico（安装器、开始菜单、Server exe、输入法列表都用它）。
# 16px 用单独画的像素对齐版 app-16.svg：原图两列竹简之间的缝缩到 16px 会糊掉。
#   assets/icon/windows/render-app-icon.sh      # 需要 rsvg-convert 与 magick
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
SRC="$ROOT/assets/icon/windows"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
rsvg-convert -w 16 -h 16 "$SRC/app-16.svg" -o "$TMP/16.png"
for size in 20 24 32 48 64 128 256; do
  rsvg-convert -w "$size" -h "$size" "$SRC/app.svg" -o "$TMP/$size.png"
done
# 48 及以下存 BMP（老的读取方只认 BMP），64 起存 PNG：magick 一律存 BMP，256 那档就有 256 KB。
for size in 16 20 24 32 48; do
  magick "$TMP/$size.png" "$TMP/$size.ico"
done
python3 -I - "$TMP" "$ROOT/apps/windows/tsf/resources/qingjian.ico" <<'PY'
import struct, sys
tmp, out = sys.argv[1], sys.argv[2]
entries = []
for size in (16, 20, 24, 32, 48):
    ico = open(f"{tmp}/{size}.ico", "rb").read()
    length, offset = struct.unpack("<II", ico[14:22])
    entries.append((size, ico[offset:offset + length]))
for size in (64, 128, 256):
    entries.append((size, open(f"{tmp}/{size}.png", "rb").read()))
offset = 6 + 16 * len(entries)
header, body = struct.pack("<HHH", 0, 1, len(entries)), b""
for size, data in entries:
    header += struct.pack("<BBBBHHII", size % 256, size % 256, 0, 0, 1, 32, len(data), offset)
    body += data
    offset += len(data)
open(out, "wb").write(header + body)
PY
magick identify "$ROOT/apps/windows/tsf/resources/qingjian.ico"
