#!/usr/bin/env bash
# 把任务栏单色图标的 SVG 栅格化成 qingjian-mono-{dark,light}.ico（任务栏缺省的单色版，无边框）。
# 与 render-app-icon.sh 同管线：16–48 存 BMP（老的读取方只认 BMP），64 起存 PNG。
#   assets/icon/windows/render-app-mono-icon.sh      # 需要 rsvg-convert 与 magick
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
SRC="$ROOT/assets/icon/windows"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
for variant in dark light; do
  for size in 16 20 24 32 48 64 128 256; do
    rsvg-convert -w "$size" -h "$size" "$SRC/app-mono-$variant.svg" -o "$TMP/m-$variant-$size.png"
  done
  for size in 16 20 24 32 48; do
    magick "$TMP/m-$variant-$size.png" "$TMP/m-$variant-$size.ico"
  done
  python3 -I - "$TMP" "$variant" "$ROOT/apps/windows/tsf/resources/qingjian-mono-$variant.ico" <<'PY'
import struct, sys
tmp, variant, out = sys.argv[1], sys.argv[2], sys.argv[3]
entries = []
for size in (16, 20, 24, 32, 48):
    ico = open(f"{tmp}/m-{variant}-{size}.ico", "rb").read()
    length, offset = struct.unpack("<II", ico[14:22])
    entries.append((size, ico[offset:offset + length]))
for size in (64, 128, 256):
    entries.append((size, open(f"{tmp}/m-{variant}-{size}.png", "rb").read()))
offset = 6 + 16 * len(entries)
header, body = struct.pack("<HHH", 0, 1, len(entries)), b""
for size, data in entries:
    header += struct.pack("<BBBBHHII", size % 256, size % 256, 0, 0, 1, 32, len(data), offset)
    body += data
    offset += len(data)
open(out, "wb").write(header + body)
PY
done
magick identify "$ROOT/apps/windows/tsf/resources/qingjian-mono-dark.ico" "$ROOT/apps/windows/tsf/resources/qingjian-mono-light.ico"
