#!/usr/bin/env bash
# //! 在 Linux 虚拟机（Debian 13）里验当前工作区的源码安装：同步源码 → 取正式数据并校验 → install.sh 装到 ~/.local
# //! → Fcitx5 插件 ctest（对着 Debian 自带的 Fcitx5，比 CI 容器里的旧）→ 用新装的 Server 替换正在跑的。图形端到端在 e2e.sh。
# //! 全 workspace 测试不在这里跑：本机 local 已经是 Linux，重复跑只是慢。
# 用法：check.sh [--skip-tests]（只装不测，给 e2e.sh 准备环境时用）
set -euo pipefail
here=$(dirname -- "${BASH_SOURCE[0]}")
# shellcheck source=../lib.sh
source "$here/../lib.sh"
use_vm linux
skip_tests=false
[[ "${1:-}" == --skip-tests ]] && skip_tests=true
vm_running || "$here/vm.sh" start
vm_wait true 300 || die '等不到虚拟机的 SSH'
# AppImage 装的和源码装的同名文件互相拒绝，appimage.sh 中途失败留下的要先清掉
vm_ssh 'test ! -e ~/.local/share/qingjian-appimage' || die '虚拟机里还装着 AppImage：先跑 tools/verify/linux/appimage.sh --cleanup'
log=$verify_out/linux-check.log
: > "$log"
remote="set -e; . ~/.cargo/env; cd ~/qingjian; export CARGO_HTTP_TIMEOUT=120 CARGO_NET_RETRY=10"
run() {
  local name=$1; shift
  step "$name"
  if vm_ssh "$remote; $*" >> "$log" 2>&1; then ok "$name"; else fail "$name，完整输出：$log"; tail -30 "$log" >&2; exit 1; fi
}

step '同步源码（不带 target / data / .git）'
vm_ssh 'mkdir -p ~/qingjian'
vm_rsync --delete --exclude /target --exclude /data --exclude .git --exclude .codegraph "$repo_root/" "$vm_user@127.0.0.1:qingjian/"
vm_rsync "$here/guest/" "$here/../guest/" "$vm_user@127.0.0.1:bin/"
ok 同步

# 本地的 data/generated 可能是旧版本（与 data.lock 对不上 install.sh 才报），所以一律按锁文件下载校验
run '取正式数据（data.lock）' 'tools/release/data-fetch.sh'
run '拉依赖' 'cargo fetch --locked'
run 'install.sh（release，装到 ~/.local）' 'CARGO_NET_OFFLINE=true bash apps/linux/scripts/install.sh'

if ! $skip_tests; then
  # ctest 要拉起真 Server，缺省找 target/debug 下的；这里不跑 cargo test 就没有它，直接用 install.sh 刚编的 release（也就是装上去的那份）
  run 'Fcitx5 插件 ctest（Debian 13 的 Fcitx5）' 'cmake -S apps/linux/fcitx5 -B target/fcitx5-test -DCMAKE_BUILD_TYPE=Debug -DBUILD_TESTING=ON -DQINGJIAN_SERVER_EXECUTABLE="$PWD/target/release/qingjian-linux-server" >/dev/null && cmake --build target/fcitx5-test --parallel 4 && cd target/fcitx5-test && env -u DBUS_SESSION_BUS_ADDRESS ctest --output-on-failure -j4 < /dev/null'
  grep -E 'tests passed' "$log" | tail -1 | sed 's/^/  /'
  run '安装脚本测试' 'python3 apps/linux/tests/installation.py'
fi

step '用新装的 Server 替换正在跑的'
vm_ssh 'pkill -x qingjian-linux- || true; sleep 1; pgrep -x xfce4-session >/dev/null || exit 0; ~/bin/insession setsid "$HOME/.local/bin/qingjian-linux-server" </dev/null >/dev/null 2>&1 & sleep 3; pgrep -f "^$HOME/.local/bin/qingjian-linux-server" >/dev/null'
# Fcitx5 只在启动时加载插件：重启一次换上新装的，顺带确认青简在输入法列表里、切得过去
vm_ssh '~/bin/insession ~/bin/use-qingjian'
ok 'Server 与 Fcitx5 已重启'
