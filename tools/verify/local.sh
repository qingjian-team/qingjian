#!/usr/bin/env bash
# //! 本机这一路：fmt、clippy、全 workspace 测试（范围与 .githooks 一致），再加两项钩子不跑的：
# //! 安装脚本测试（apps/linux/tests/installation.py）与设置程序截图脚本（--png，Linux 上曾中途失败没人发现）。
# 用法：local.sh
set -euo pipefail
# shellcheck source=lib.sh
source "$(dirname -- "${BASH_SOURCE[0]}")/lib.sh"
cd "$repo_root"
case "$(uname -s)" in
  Darwin) scope=(--workspace) ;;
  *) scope=(--workspace --exclude qingjian-macos) ;;
esac
run() {
  local name=$1; shift
  step "$name"
  if "$@" > "$verify_out/local-$name.log" 2>&1; then ok "$name"; else fail "$name，完整输出：$verify_out/local-$name.log"; tail -30 "$verify_out/local-$name.log" >&2; exit 1; fi
}
run fmt cargo fmt --all --check
run clippy cargo clippy "${scope[@]}" --all-targets --locked -- -D warnings
run test cargo test "${scope[@]}" --locked
awk '/^test result/ {p += $4; f += $6} END {print "  " p " 个测试通过，" f " 个失败"}' "$verify_out/local-test.log"
if [[ "$(uname -s)" == Linux ]]; then
  run installation python3 apps/linux/tests/installation.py
fi
# 截图脚本拷一份配置到输出目录再点。--config 给一个不存在的文件让它用缺省配置：不给的话读的是本机真配置
# （macOS / Windows 不认 XDG），结果就取决于这台机器配了什么——Mac 测试机上曾因真配置里的 traditional = "off" 整个读不进
# 在 run 的 if 里调用，set -e 不生效，所以每一步的失败都要自己传回去
snapshot() {
  local home code=0
  home=$(mktemp -d)
  cargo build -q --locked -p qingjian-settings || return
  rm -rf "$verify_out/settings-png"
  XDG_CONFIG_HOME=$home/config XDG_DATA_HOME=$home/data \
    target/debug/qingjian-settings --config "$home/none.toml" --png "$verify_out/settings-png" || code=$?
  rm -rf "$home"
  return "$code"
}
run settings-png snapshot
# 没截出图就算失败：脚本中途出错时进程照样可能退出 0（比如磁盘写满）
shopt -s nullglob
shots=("$verify_out"/settings-png/*.png)
((${#shots[@]} > 0)) || { fail '设置程序一张截图都没出'; exit 1; }
echo "  截图：$verify_out/settings-png（${#shots[@]} 张）"
