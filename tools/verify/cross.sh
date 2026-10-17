#!/usr/bin/env bash
# //! 在本机对别的平台做类型检查（clippy，不链接、不出可执行文件）：macOS、FreeBSD、Windows。
# //! 范围按「那个平台上的开发者跑 pre-commit 时编的范围」：#628 就是 Linux 上只查了 IMK 壳，Mac 上整个 workspace 编不过。
# 用法：cross.sh [darwin] [freebsd] [windows]（缺省三个都跑）；没装的 target 先 rustup target add 再跑。
set -euo pipefail
# shellcheck source=lib.sh
source "$(dirname -- "${BASH_SOURCE[0]}")/lib.sh"
cd "$repo_root"

# onig_sys（candle → tokenizers）要编 C 库；给一个假的 pkg-config 条目让它改走「系统库」，只做检查不链接，库在不在无所谓
onig=$repo_root/target/darwin-check/onig
if [[ ! -f "$onig/oniguruma.pc" ]]; then
  mkdir -p "$onig/include"
  : > "$onig/include/oniguruma.h"
  printf 'prefix=%s\nincludedir=${prefix}/include\nName: oniguruma\nDescription: only for cargo check, never linked\nVersion: 6.9.9\nLibs: -lonig\nCflags: -I${includedir}\n' \
    "$onig" > "$onig/oniguruma.pc"
fi
fake_onig=(env PKG_CONFIG_PATH="$onig" PKG_CONFIG_ALLOW_CROSS=1 RUSTONIG_SYSTEM_LIBONIG=1)

check() {
  local name=$1 target=$2; shift 2
  step "$name（$target）"
  rustup target list --installed | grep -qx "$target" || rustup target add "$target"
  if "${fake_onig[@]}" cargo clippy --locked --target "$target" "$@" --all-targets -- -D warnings \
      > "$verify_out/cross-$name.log" 2>&1; then
    ok "$name"
  else
    grep -E '^(error|warning)' "$verify_out/cross-$name.log" | sort | uniq -c | head -20 >&2
    fail "$name，完整输出：$verify_out/cross-$name.log"
    return 1
  fi
}

targets=("$@")
((${#targets[@]})) || targets=(darwin freebsd windows)
status=0
for target in "${targets[@]}"; do
  case "$target" in
    # 与 Mac 上的 pre-commit 一致：整个 workspace
    darwin) check darwin aarch64-apple-darwin --workspace || status=1 ;;
    # FreeBSD 走 Linux 那套前端：Server、Fcitx5 侧、设置程序；IMK 壳在非 Apple 平台是硬 compile_error!
    freebsd) check freebsd x86_64-unknown-freebsd --workspace --exclude qingjian-macos || status=1 ;;
    # 与 ci.yml 的 windows job 同一范围；gnu 目标在 Linux 上也能检查，msvc 要 Windows SDK
    windows) check windows x86_64-pc-windows-gnu -p qingjian-windows-server -p qingjian-windows-tsf \
      -p qingjian-windows-settings -p qingjian-settings || status=1 ;;
    *) die "不认识的目标：$target（darwin / freebsd / windows）" ;;
  esac
done
exit "$status"
