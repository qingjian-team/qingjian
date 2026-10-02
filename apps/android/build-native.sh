#!/usr/bin/env bash
# 用调用者的 Rust 工具链和 NDK 构建 ARM64 JNI，保留 16 KiB 页对齐。
set -euo pipefail

repo="$(cd "$(dirname "$0")/../.." && pwd)"
ndk="${QINGJIAN_NDK:-${ANDROID_NDK_HOME:-}}"
out="${QINGJIAN_ANDROID_NATIVE_OUT:-$repo/apps/android/build/native/arm64-v8a}"
linker="$ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android26-clang"
[[ -n "$ndk" && -x "$linker" ]] || { echo '请设置包含 Linux 工具链的 QINGJIAN_NDK 或 ANDROID_NDK_HOME' >&2; exit 2; }
command -v cargo >/dev/null || { echo 'PATH 中找不到 cargo' >&2; exit 2; }
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$linker"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS='-C link-arg=-Wl,-z,max-page-size=16384'
cd "$repo"
cargo test --locked -p qingjian-android
cargo build --locked --release --target aarch64-linux-android -p qingjian-android
target="${CARGO_TARGET_DIR:-$repo/target}"
mkdir -p "$out"
cp "$target/aarch64-linux-android/release/libqingjian_android.so" "$out/libqingjian_android.so"
