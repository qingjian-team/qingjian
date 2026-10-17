#!/usr/bin/env bash
# //! Linux 虚拟机（Debian 13 + XFCE，X11）里的图形端到端：对 GTK3 / Qt6 编辑器与 Firefox 逐条跑 tools/verify/cases.tsv，
# //! 读回上屏文本比对；每个应用截一张图。结果在 target/verify/linux-e2e/：summary.md、results.tsv、*.png。
# 用法：e2e.sh [应用 …]，例如 e2e.sh mousepad；缺省 mousepad featherpad firefox。先跑过 check.sh（或 check.sh --skip-tests）。
# 下面传给 vm_ssh 的 ~ 都是要在虚拟机那边展开的
# shellcheck disable=SC2088
set -euo pipefail
here=$(dirname -- "${BASH_SOURCE[0]}")
# shellcheck source=../lib.sh
source "$here/../lib.sh"
use_vm linux
out=$verify_out/linux-e2e
rm -rf "$out" && mkdir -p "$out"

apps=("$@")
((${#apps[@]})) || apps=(mousepad featherpad firefox)

vm_running || die '虚拟机没在运行：tools/verify/linux/vm.sh start'
vm_ssh 'test -x ~/.local/bin/qingjian-linux-server' || die '虚拟机里还没装青简：先跑 tools/verify/linux/check.sh'
vm_wait 'pgrep -x xfce4-session >/dev/null && pgrep -x fcitx5 >/dev/null' 120 || die '虚拟机里没有 XFCE 会话'
vm_rsync "$here/guest/" "$here/../guest/" "$vm_user@127.0.0.1:bin/"
vm_rsync "$here/../cases.tsv" "$vm_user@127.0.0.1:bin/cases.tsv"

printf '环境\t应用\t结果\t按键\t期望\t实际\n' > "$out/results.tsv"
status=0
for app in "${apps[@]}"; do
  step "xfce · $app"
  result=$(vm_ssh "~/bin/insession ~/bin/e2e-app.sh x11 $app ~/bin/cases.tsv" 2>&1) || status=1
  vm_shot "$out/xfce-$app.png"
  e2e_collect xfce "$app" "$result" "$out/results.tsv" || status=1
  vm_ssh "pkill -x $app; pkill -x firefox-esr; sleep 2; pkill -KILL -x $app; pkill -KILL -x firefox-esr" || true
done

e2e_summary 'Linux 图形端到端（Debian 13）' "$out/results.tsv" > "$out/summary.md"
step '汇总'
cat "$out/summary.md"
echo "截图与逐条结果：$out"
exit "$status"
