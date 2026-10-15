#!/usr/bin/env bash
# //! 用户目录安装默认面板链路；Server 由用户手动启动。
set -euo pipefail
repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)
install_prefix="$HOME/.local"
build_profile=release
cmake_type=Release
sample=false
while (($#)); do
  case "$1" in
    --prefix) install_prefix=${2:?--prefix 需要路径}; shift 2 ;;
    --debug) build_profile=debug; cmake_type=Debug; shift ;;
    --sample) sample=true; shift ;;
    --help) echo '用法：install.sh [--prefix 绝对用户目录] [--debug] [--sample]'; exit 0 ;;
    *) echo "未知参数：$1" >&2; exit 2 ;;
  esac
done
[[ "$install_prefix" = /* && "$install_prefix" != / ]] || { echo '需要非根绝对安装路径' >&2; exit 2; }
# 缺依赖先说清楚，别等编译到一半才报一长串错。
missing=()
for tool in cargo cmake c++ pkg-config python3; do
  command -v "$tool" >/dev/null 2>&1 || missing+=("$tool")
done
if command -v pkg-config >/dev/null 2>&1; then
  for module in openssl Fcitx5Core Fcitx5Utils Fcitx5Config nlohmann_json; do
    pkg-config --exists "$module" || missing+=("$module")
  done
fi
if ((${#missing[@]})); then
  echo "缺少依赖：${missing[*]}" >&2
  echo 'Debian / Ubuntu：sudo apt install cmake g++ pkg-config python3 libssl-dev libfcitx5core-dev libfcitx5utils-dev libfcitx5config-dev nlohmann-json3-dev；Rust 见 https://rustup.rs' >&2
  exit 1
fi
install_prefix=$(realpath -m -- "$install_prefix")
cargo_output=$(realpath -m -- "${CARGO_TARGET_DIR:-$repo_root/target}")
cargo_args=(build --manifest-path "$repo_root/Cargo.toml" --target-dir "$cargo_output" -p qingjian-linux-server -p qingjian-settings --locked)
[[ "$build_profile" != release ]] || cargo_args+=(--release)
cargo "${cargo_args[@]}"
cmake_output="$repo_root/target/fcitx5-install-$build_profile"
cmake -S "$repo_root/apps/linux/fcitx5" -B "$cmake_output" "-DCMAKE_BUILD_TYPE=$cmake_type" -DBUILD_TESTING=OFF
cmake --build "$cmake_output" --parallel "${CMAKE_BUILD_PARALLEL_LEVEL:-2}"
python3 "$repo_root/apps/linux/scripts/files.py" install "$install_prefix" "$repo_root" "$cargo_output/$build_profile/qingjian-linux-server" "$cargo_output/$build_profile/qingjian-settings" "$cmake_output/qingjian.so" "$sample"
printf '已安装。手动启动：%s/bin/qingjian-linux-server\n重启 Fcitx5，在配置工具取消“仅显示当前语言”后添加“青简”。\n设置：应用菜单里的“青简设置”，或 %s/bin/qingjian-settings\n' "$install_prefix" "$install_prefix"
