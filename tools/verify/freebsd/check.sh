#!/usr/bin/env bash
# //! 在 FreeBSD 虚拟机里验当前工作区：同步源码 → 取正式数据并校验 → install.sh 装到 ~/.local → 全 workspace 测试
# //! → Fcitx5 插件 ctest → 安装脚本测试 → 用新装的 Server 替换正在跑的。图形端到端在 e2e.sh。
# 用法：check.sh [--skip-tests]（只装不测，给 e2e.sh 准备环境时用）
set -euo pipefail
here=$(dirname -- "${BASH_SOURCE[0]}")
# shellcheck source=../lib.sh
source "$here/../lib.sh"
skip_tests=false
[[ "${1:-}" == --skip-tests ]] && skip_tests=true
vm_running || "$here/vm.sh" start
vm_wait true 300 || die '等不到虚拟机的 SSH'
log=$verify_out/freebsd-check.log
: > "$log"
# cargo 在虚拟机里拉 crates.io 索引偶尔超时：先单独 fetch，超时调长、多重试，之后离线编
remote="set -e; . ~/.cargo/env; cd ~/qingjian; export CARGO_HTTP_TIMEOUT=120 CARGO_NET_RETRY=10"
run() {
  local name=$1; shift
  step "$name"
  if vm_ssh "$remote; $*" >> "$log" 2>&1; then ok "$name"; else fail "$name，完整输出：$log"; tail -30 "$log" >&2; exit 1; fi
}

step '同步源码（不带 target / data / .git）'
vm_ssh 'mkdir -p ~/qingjian'
vm_rsync --delete --exclude /target --exclude /data --exclude .git --exclude .codegraph "$repo_root/" "$vm_user@127.0.0.1:qingjian/"
ok 同步

# 本地的 data/generated 可能是旧版本（与 data.lock 对不上 install.sh 才报），所以一律按锁文件下载校验
run '取正式数据（data.lock）' 'tools/release/data-fetch.sh'
run '拉依赖' 'cargo fetch --locked'
run 'install.sh（release，装到 ~/.local）' 'CARGO_NET_OFFLINE=true bash apps/linux/scripts/install.sh'

if ! $skip_tests; then
  run 'cargo test（整个 workspace，排除 IMK 壳）' 'CARGO_NET_OFFLINE=true cargo test --workspace --exclude qingjian-macos --locked'
  grep -E '^test result' "$log" | awk '{p += $4; f += $6} END {print "  " p " 个测试通过，" f " 个失败"}'
  run 'Fcitx5 插件 ctest' 'cmake -S apps/linux/fcitx5 -B target/fcitx5-test -DCMAKE_BUILD_TYPE=Debug -DBUILD_TESTING=ON >/dev/null && cmake --build target/fcitx5-test --parallel 8 && cd target/fcitx5-test && ctest --output-on-failure -j4'
  grep -E 'tests passed' "$log" | tail -1 | sed 's/^/  /'
  run '安装脚本测试' 'python3 apps/linux/tests/installation.py'
fi

step '用新装的 Server 替换正在跑的'
vm_ssh 'pkill -f "^$HOME/.local/bin/qingjian-linux-server" || true; sleep 1; pgrep -x xfce4-session >/dev/null || pgrep -x plasmashell >/dev/null || exit 0; ~/bin/insession daemon -f "$HOME/.local/bin/qingjian-linux-server"; sleep 3; pgrep -f "^$HOME/.local/bin/qingjian-linux-server" >/dev/null'
ok 'Server 已重启'
