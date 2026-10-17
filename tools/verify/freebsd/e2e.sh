#!/usr/bin/env bash
# //! FreeBSD 虚拟机里的图形端到端：在 KDE Plasma（X11）、XFCE（X11）、嵌套 sway（Wayland）里，
# //! 对 GTK3 / Qt / Qt6 编辑器与 Firefox 逐条跑 tools/verify/cases.tsv，读回上屏文本比对；每个应用截一张图。
# //! 结果在 target/verify/freebsd-e2e/：summary.md（汇总表）、results.tsv（逐条）、*.png（截图）。
# 用法：e2e.sh [环境:应用 …]，例如 e2e.sh xfce:mousepad wayland:kate；缺省跑全矩阵（约 15 分钟）。
# 先跑过 check.sh（或 check.sh --skip-tests）把当前工作区装进虚拟机。
# 下面传给 vm_ssh 的 ~ 都是要在虚拟机那边展开的
# shellcheck disable=SC2088
set -euo pipefail
here=$(dirname -- "${BASH_SOURCE[0]}")
# shellcheck source=../lib.sh
source "$here/../lib.sh"
out=$verify_out/freebsd-e2e
rm -rf "$out" && mkdir -p "$out"

matrix=("$@")
((${#matrix[@]})) || matrix=(
  plasmax11:kate plasmax11:firefox
  xfce:mousepad xfce:featherpad xfce:kate xfce:firefox
  wayland:mousepad wayland:kate wayland:firefox
)

vm_running || die '虚拟机没在运行：tools/verify/freebsd/vm.sh start'
vm_ssh 'test -x ~/.local/bin/qingjian-linux-server' || die '虚拟机里还没装青简：先跑 tools/verify/freebsd/check.sh'
vm_rsync "$here/guest/" "$here/../guest/" "$vm_user@127.0.0.1:bin/"
vm_rsync "$here/../cases.tsv" "$vm_user@127.0.0.1:bin/cases.tsv"

current_session=
nested=false
ensure_session() {
  local want=$1
  [[ "$want" == wayland ]] && want=xfce
  if [[ "$current_session" != "$want" ]]; then
    $nested && { vm_ssh '~/bin/insession ~/bin/nested-sway.sh stop' || true; nested=false; }
    step "切到 $want 会话"
    vm_ssh root@ "sh /home/$vm_user/bin/session.sh $want"
    current_session=$want
  fi
  if [[ "$1" == wayland ]] && ! $nested; then
    step '起嵌套 sway（Wayland）'
    vm_ssh '~/bin/insession ~/bin/nested-sway.sh start' >/dev/null || die '嵌套 sway 没起来，看虚拟机里的 /tmp/qingjian-nested-sway.log'
    nested=true
  fi
}

printf '环境\t应用\t结果\t按键\t期望\t实际\n' > "$out/results.tsv"
status=0
for entry in "${matrix[@]}"; do
  env=${entry%%:*} app=${entry#*:}
  mode=x11; [[ "$env" == wayland ]] && mode=wayland
  ensure_session "$env"
  step "$env · $app"
  result=$(vm_ssh "~/bin/insession ~/bin/e2e-app.sh $mode $app ~/bin/cases.tsv" 2>&1) || status=1
  vm_shot "$out/$env-$app.png"
  e2e_collect "$env" "$app" "$result" "$out/results.tsv" || status=1
  vm_ssh "pkill -x $app; sleep 2; pkill -KILL -x $app" || true
done
$nested && vm_ssh '~/bin/insession ~/bin/nested-sway.sh stop' || true
# 回到缺省的 XFCE，下次手工上去看是熟悉的样子
[[ "$current_session" == xfce ]] || vm_ssh root@ "sh /home/$vm_user/bin/session.sh xfce" >/dev/null || true

e2e_summary "FreeBSD 图形端到端" "$out/results.tsv" > "$out/summary.md"
step '汇总'
cat "$out/summary.md"
echo "截图与逐条结果：$out"
exit "$status"
