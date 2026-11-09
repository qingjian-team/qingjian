#!/usr/bin/env bash
# 把编好的 qingjian-clipboard 装成 QingjianClipboard.app。输入法打包（apps/macos/scripts/bundle.sh，装进 Contents/Applications/）
# 与剪贴板单独打包（apps/clipboard/macos/build.sh）共用这一份，两边的包长得一样。不签名，由调用方整包签。
#
#   apps/clipboard/macos/assemble.sh <qingjian-clipboard 二进制> <输出的 .app> <构建号> <Qingjian.icns>
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
BINARY="$1"
APP="$2"
BUILD_NUMBER="$3"
ICNS="$4"

# 版本号来自 apps/clipboard/Cargo.toml；-dev 版接 git 短哈希（工作区有改动加 +），与输入法的规矩一致
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/apps/clipboard/Cargo.toml" | head -1)"
if [[ "$VERSION" == *-dev ]]; then
  REV="$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"
  if [[ -n "$(git -C "$ROOT" status --porcelain 2>/dev/null)" ]]; then REV="${REV}+"; fi
  VERSION="${VERSION}-${REV}"
fi

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BINARY" "$APP/Contents/MacOS/qingjian-clipboard"
cp "$ROOT/apps/clipboard/macos/Info.plist" "$APP/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $VERSION" \
  -c "Set :CFBundleVersion $BUILD_NUMBER" "$APP/Contents/Info.plist"
cp "$ICNS" "$APP/Contents/Resources/Qingjian.icns"
printf 'APPL????' > "$APP/Contents/PkgInfo"
echo "$VERSION"
