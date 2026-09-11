#!/usr/bin/env bash
# 把 qingjian-macos 打包成 Qingjian.app。
#
#   scripts/bundle.sh            # 只打包到 target/Qingjian.app
#   scripts/bundle.sh --install  # 打包并安装到 ~/Library/Input Methods/，杀掉旧进程（开发用）
#   scripts/bundle.sh --pkg      # 打包并做成 target/pkg/Qingjian-<版本>-<arm64|x86_64>.pkg（分发给测试者）
#
# 架构：缺省编译本机架构；QINGJIAN_TARGET=x86_64-apple-darwin（或 aarch64-apple-darwin）交叉编译另一种，
# 先 `rustup target add` 一次。CI 在 Apple Silicon runner 上两个都打（.github/workflows/release.yml）。
# 产品数据：data/generated/ 里有 dict.qj 就用自建词库（TSV 比 .qj 新会重打），没有就退回 assets/sample/ 样例。
#
# 签名与公证都由环境变量决定，没设就 ad-hoc 签名、pkg 不签（本机自用够了，分发给别人会被 Gatekeeper 拦，
# 对方要在「系统设置 → 隐私与安全性」里点「仍要打开」）：
#   QINGJIAN_SIGN_IDENTITY       "Developer ID Application: …"   给 .app 签名（hardened runtime）
#   QINGJIAN_INSTALLER_IDENTITY  "Developer ID Installer: …"     给 .pkg 签名
#   QINGJIAN_NOTARY_PROFILE      notarytool store-credentials 存的 keychain profile 名，设了就公证并钉票据
#
# 首次 --install 后要在「系统设置 → 键盘 → 输入法」里添加「青简」；输入法列表不刷新就注销再登录。
# pkg 装的不用：postinstall 会以登录用户身份跑 `qingjian-macos --register` 注册并启用。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
APP_NAME="Qingjian"
BIN_NAME="qingjian-macos"
PROFILE="${PROFILE:-release}"
APP="$ROOT/target/$APP_NAME.app"
INSTALL_DIR="$HOME/Library/Input Methods"
# 目标三元组为空就是本机；架构名按 pkg 文件名与 distribution.xml 的 hostArchitectures 用的写法（arm64 / x86_64）
TARGET="${QINGJIAN_TARGET:-}"
case "${TARGET:-$(uname -m)}" in
  aarch64-apple-darwin|arm64) ARCH="arm64" ;;
  x86_64-apple-darwin|x86_64) ARCH="x86_64" ;;
  *) echo "不认识的架构: ${TARGET:-$(uname -m)}" >&2; exit 1 ;;
esac

cd "$ROOT"
# 构建标识进「关于」页与诊断信息：git 短哈希（工作区有改动加 +）与日期；编译期 option_env! 读
GIT_REV="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
if [[ -n "$(git status --porcelain 2>/dev/null)" ]]; then GIT_REV="${GIT_REV}+"; fi
export QINGJIAN_BUILD="${GIT_REV} · $(date +%Y-%m-%d)"
BUILD_ARGS=(-p "$BIN_NAME")
[[ "$PROFILE" == "release" ]] && BUILD_ARGS+=(--release)
BIN_DIR="target/$PROFILE"
if [[ -n "$TARGET" ]]; then
  BUILD_ARGS+=(--target "$TARGET")
  BIN_DIR="target/$TARGET/$PROFILE"
fi
cargo build "${BUILD_ARGS[@]}"

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN_DIR/$BIN_NAME" "$APP/Contents/MacOS/$BIN_NAME"
cp apps/macos/Info.plist "$APP/Contents/Info.plist"
# 版本号来自 apps/macos/Cargo.toml（各平台壳版本号独立，不跟 workspace 走），构建号用提交数（单调递增，pkg 升级判断靠它）
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' apps/macos/Cargo.toml | head -1)"
BUILD_NUMBER="$(git rev-list --count HEAD 2>/dev/null || echo 1)"
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $VERSION" \
  -c "Set :CFBundleVersion $BUILD_NUMBER" "$APP/Contents/Info.plist"
# 卸载脚本随包，装了 pkg 的用户从 Resources 里运行
cp apps/macos/scripts/uninstall.sh "$APP/Contents/Resources/uninstall.sh"
# 输入源名字按系统语言本地化（中文系统显示「青简」，其他显示 Qingjian）
cp -R apps/macos/resources/*.lproj "$APP/Contents/Resources/"
# 词库与释义表打进 Resources。data/generated/ 里有生成好的产品数据（自建词库 + 语言模型 + LLM 释义表）就用它，
# 否则用 assets/sample/ 的样例。
cp assets/sample/*.tsv "$APP/Contents/Resources/"
# emoji 表（Unicode CLDR，可发布）
cp assets/emoji/*.tsv "$APP/Contents/Resources/"
# 词汇等级表（CEFR-J / Octanove / JLPT，见 assets/levels/README.md），「统计」页按级数词汇
cp assets/levels/levels-*.tsv "$APP/Contents/Resources/"
if [[ -f data/generated/dict.tsv || -f data/generated/dict.qj ]]; then
  # 词库与语言模型打成 .qj（mmap 直接用），TSV 比 .qj 新时重新打包；只有 .qj（CI 从数据包解出来的）就直接用
  if [[ -f data/generated/dict.tsv && ( ! -f data/generated/dict.qj || data/generated/dict.tsv -nt data/generated/dict.qj ) ]]; then
    cargo run --release -q -p qingjian-dict-convert -- pack dict --name "青简基础词库" \
      --license "MIT AND Unicode-3.0" --attribution "通用规范汉字表；现代汉语常用词表（liuxilu 校对版）；THUOCL（清华大学自然语言处理实验室，MIT）；读音 Unihan（Unicode）" \
      --source https://github.com/qingjian-team/qingjian/tree/main/assets/lexicon
  fi
  if [[ -f data/generated/lm-bigram.tsv && ( ! -f data/generated/lm.qj || data/generated/lm-bigram.tsv -nt data/generated/lm.qj ) ]]; then
    cargo run --release -q -p qingjian-dict-convert -- pack lm --name "青简语言模型（中文维基 + LCCC，青简词库分词）" \
      --license "CC-BY-SA-4.0 AND MIT" --attribution "中文维基百科（CC BY-SA 4.0）；LCCC（清华大学 CoAI，MIT）"
  fi
  cp data/generated/dict.qj "$APP/Contents/Resources/"
  # 领域词库（lexicon 拆出的 dicts/*.qj）随包放 Resources/dicts/，缺省只开成语，偏好设置「词库」页可勾选
  if ls data/generated/dicts/*.qj >/dev/null 2>&1; then
    mkdir -p "$APP/Contents/Resources/dicts"
    cp data/generated/dicts/*.qj "$APP/Contents/Resources/dicts/"
  fi
  [[ -f data/generated/lm.qj ]] && cp data/generated/lm.qj "$APP/Contents/Resources/"
  # 本地整句模型（字级 Transformer，tools/lm-train 导出的三件套）随包放 Resources/model/；没有就不重排
  model_dir="${QINGJIAN_MODEL_DIR:-data/lm-train/export/full-small}"
  if [[ -f "$model_dir/model.safetensors" ]]; then
    mkdir -p "$APP/Contents/Resources/model"
    cp "$model_dir/model.safetensors" "$model_dir/config.json" "$model_dir/vocab.json" "$APP/Contents/Resources/model/"
    chmod 644 "$APP/Contents/Resources/model/"*
    echo "打包本地整句模型：$model_dir"
  fi
  # 释义表打成 .qj（TSV 比 .qj 新时重打），英文词表仍是 TSV
  for lang in en ja zh; do
    src="assets/glossary/glossary-$lang.tsv"
    out="data/generated/glossary-$lang.qj"
    [[ -f "$src" ]] || continue
    if [[ ! -f "$out" || "$src" -nt "$out" ]]; then
      cargo run --release -q -p qingjian-dict-convert -- pack glossary --language "$lang" --input "$src" \
        --name "青简释义表（${lang}）" --license "MIT" --attribution "LLM 生成（DeepSeek），qingjian-gloss-gen"
    fi
    cp "$out" "$APP/Contents/Resources/"
  done
  for f in assets/lexicon/english.tsv data/generated/english.tsv; do
    [[ -f "$f" ]] && cp "$f" "$APP/Contents/Resources/"
  done
  echo "使用 data/generated/ 的产品数据（自建词库）"
fi
printf 'APPL????' > "$APP/Contents/PkgInfo"

# 图标：从 assets/icon/logo.png 生成 .icns（应用图标）与多分辨率 tiff（输入法菜单图标）
ICONSET="$ROOT/target/Qingjian.iconset"
rm -rf "$ICONSET" && mkdir -p "$ICONSET"
for size in 16 32 128 256 512; do
  sips -z $size $size assets/icon/logo.png --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
  double=$((size * 2))
  sips -z $double $double assets/icon/logo.png --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/Qingjian.icns"
tiffutil -cathidpicheck "$ICONSET/icon_16x16.png" "$ICONSET/icon_16x16@2x.png" \
  -out "$APP/Contents/Resources/qingjian-menu.tiff" >/dev/null
# Apple Silicon 上未签名的二进制不会被系统加载。有 Developer ID 证书就正式签（开 hardened runtime，公证要求），
# 没有就 ad-hoc 签名，本机自用够了
if [[ -n "${QINGJIAN_SIGN_IDENTITY:-}" ]]; then
  codesign --force --deep --options runtime --timestamp --sign "$QINGJIAN_SIGN_IDENTITY" "$APP"
  echo "已用 Developer ID 签名: $QINGJIAN_SIGN_IDENTITY"
else
  codesign --force --deep --sign - "$APP"
fi
echo "打包完成: ${APP}（版本 ${VERSION}，构建 ${BUILD_NUMBER}，${ARCH}）"

if [[ "${1:-}" == "--pkg" ]]; then
  # 每个架构一个工作目录，成品都放 target/pkg/，两个架构接着打互不覆盖
  PKG="$ROOT/target/pkg/$APP_NAME-$VERSION-$ARCH.pkg"
  PKG_DIR="$ROOT/target/pkg/$ARCH"
  rm -rf "$PKG_DIR"
  mkdir -p "$PKG_DIR/root" "$PKG_DIR/resources"
  # 不带扩展属性复制，否则载荷里全是 ._ 元数据文件
  ditto --noextattr --norsrc --noacl "$APP" "$PKG_DIR/root/$APP_NAME.app"
  # 组件描述里关掉 bundle 重定位：否则机器上别处已有同 bundle id 的 .app（比如 ~/Library 下的开发副本）时，
  # 安装器会把新版装到那里而不是 /Library/Input Methods
  pkgbuild --analyze --root "$PKG_DIR/root" "$PKG_DIR/component.plist" >/dev/null
  /usr/libexec/PlistBuddy -c "Set :0:BundleIsRelocatable false" "$PKG_DIR/component.plist"
  pkgbuild --root "$PKG_DIR/root" --component-plist "$PKG_DIR/component.plist" \
    --install-location "/Library/Input Methods" --scripts apps/macos/pkg/scripts \
    --identifier app.qingjian.inputmethod --version "$VERSION" "$PKG_DIR/$APP_NAME-component.pkg" >/dev/null
  cp apps/macos/pkg/resources/*.html "$PKG_DIR/resources/"
  cp LICENSE "$PKG_DIR/resources/license.txt"
  # 二进制只有一种架构，hostArchitectures 限定只在对应机器上装；另一种架构用 QINGJIAN_TARGET 再打一份
  sed -e "s/@VERSION@/$VERSION/g" -e "s/@ARCH@/$ARCH/g" apps/macos/pkg/distribution.xml > "$PKG_DIR/distribution.xml"
  SIGN_ARGS=()
  if [[ -n "${QINGJIAN_INSTALLER_IDENTITY:-}" ]]; then
    SIGN_ARGS=(--sign "$QINGJIAN_INSTALLER_IDENTITY" --timestamp)
  fi
  # bash 3.2 下空数组展开会撞 set -u，用 ${arr[@]+"${arr[@]}"} 写法
  productbuild --distribution "$PKG_DIR/distribution.xml" --package-path "$PKG_DIR" \
    --resources "$PKG_DIR/resources" ${SIGN_ARGS[@]+"${SIGN_ARGS[@]}"} "$PKG" >/dev/null
  if [[ -n "${QINGJIAN_NOTARY_PROFILE:-}" ]]; then
    xcrun notarytool submit "$PKG" --keychain-profile "$QINGJIAN_NOTARY_PROFILE" --wait
    xcrun stapler staple "$PKG"
    echo "已公证并钉上票据"
  elif [[ -z "${QINGJIAN_INSTALLER_IDENTITY:-}" ]]; then
    echo "注意: pkg 未签名未公证，测试者首次打开要在「系统设置 → 隐私与安全性」里点「仍要打开」"
  fi
  echo "pkg: $PKG"
  shasum -a 256 "$PKG"
fi

if [[ "${1:-}" == "--install" ]]; then
  if [[ -d "/Library/Input Methods/$APP_NAME.app" ]]; then
    echo "注意: /Library/Input Methods/$APP_NAME.app 也装着一份（pkg 装的），两份同 id 会互相顶；先跑 scripts/uninstall.sh"
  fi
  mkdir -p "$INSTALL_DIR"
  rm -rf "$INSTALL_DIR/$APP_NAME.app"
  cp -R "$APP" "$INSTALL_DIR/$APP_NAME.app"
  # 系统会在下次切换到该输入法时重新拉起进程
  pkill -x "$BIN_NAME" 2>/dev/null || true
  echo "已安装到: $INSTALL_DIR/$APP_NAME.app"
  echo "日志: ~/Library/Logs/Qingjian/"
fi
