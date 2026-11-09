#!/usr/bin/env bash
# 打青简剪贴板的 macOS 单独包：release 构建 → 装成 QingjianClipboard.app → 签名 → 压成 zip。
# 平时剪贴板随输入法的包走（apps/macos/scripts/bundle.sh），这个包发在 qingjian-team/qingjian-clipboard，给不装输入法的人。
#
#   apps/clipboard/macos/build.sh                                    # 本机架构
#   QINGJIAN_TARGET=x86_64-apple-darwin apps/clipboard/macos/build.sh  # 交叉编译另一种（aarch64-apple-darwin 同理）
#
# 有 QINGJIAN_SIGN_IDENTITY 就用 Developer ID 签（开 hardened runtime），否则 ad-hoc 签。
# 成品在 target/installer/qingjian-clipboard-<版本>-macos-<arm64|x86_64>.zip，最后一行打印它的路径。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT"

TARGET="${QINGJIAN_TARGET:-}"
case "${TARGET:-$(uname -m)}" in
  aarch64-apple-darwin | arm64) ARCH=arm64 ;;
  x86_64-apple-darwin | x86_64) ARCH=x86_64 ;;
  *) echo "不认识的架构: ${TARGET:-$(uname -m)}" >&2; exit 1 ;;
esac

BUILD_ARGS=(--release --locked -p qingjian-clipboard-app)
BIN_DIR="target/release"
if [[ -n "$TARGET" ]]; then
  BUILD_ARGS+=(--target "$TARGET")
  BIN_DIR="target/$TARGET/release"
fi
cargo build "${BUILD_ARGS[@]}"

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
apps/macos/scripts/make-icns.sh "$STAGE/Qingjian.icns"
APP="$STAGE/QingjianClipboard.app"
BUILD_NUMBER="$(git rev-list --count HEAD 2>/dev/null || echo 1)"
VERSION="$(apps/clipboard/macos/assemble.sh "$BIN_DIR/qingjian-clipboard" "$APP" "$BUILD_NUMBER" "$STAGE/Qingjian.icns")"

# 仓库在 iCloud 同步的目录时新建的文件带 Finder 扩展属性，codesign 会拒
xattr -cr "$APP"
if [[ -n "${QINGJIAN_SIGN_IDENTITY:-}" ]]; then
  codesign --force --options runtime --timestamp --sign "$QINGJIAN_SIGN_IDENTITY" "$APP"
else
  codesign --force --sign - "$APP"
fi

mkdir -p target/installer
ZIP="target/installer/qingjian-clipboard-${VERSION}-macos-${ARCH}.zip"
rm -f "$ZIP"
# ditto 保留签名与扩展属性，访达双击解压出来的 .app 才验得过签名
ditto -c -k --keepParent "$APP" "$ZIP"
echo "$ZIP"
