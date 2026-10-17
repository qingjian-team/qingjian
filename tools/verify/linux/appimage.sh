#!/usr/bin/env bash
# //! Linux 虚拟机里实测 AppImage，走一遍用户会碰到的全部场景：从带空格与中文的路径 install → 打字 → 从菜单打开设置
# //! → 重启后靠登录自启直接打字 → 插件过期时 Server 自己同步 → 重装 → uninstall → 再重启确认干净。结果在 target/verify/linux-appimage/。
# 用法：appimage.sh [AppImage 文件] | --cleanup
#   不给文件就用 target/appimage/ 里最新的（apps/linux/appimage/build.sh 打的）；--cleanup 清掉中途失败留下的 AppImage 安装。
# 源码安装（check.sh）与 AppImage 的同名文件互相拒绝，这里会先卸掉源码安装（用户数据保留）；之后要再测源码安装重跑 check.sh 即可。
# 下面传给 vm_ssh 的 ~ 都是要在虚拟机那边展开的
# shellcheck disable=SC2088
set -euo pipefail
here=$(dirname -- "${BASH_SOURCE[0]}")
# shellcheck source=../lib.sh
source "$here/../lib.sh"
use_vm linux
out=$verify_out/linux-appimage
steps='~/bin/insession ~/bin/appimage-steps.sh'

vm_running || "$here/vm.sh" start
vm_wait 'pgrep -x xfce4-session >/dev/null && pgrep -x fcitx5 >/dev/null' 300 || die '虚拟机里没有 XFCE 会话'
vm_rsync "$here/guest/" "$here/../guest/" "$vm_user@127.0.0.1:bin/"
vm_rsync "$here/../cases.tsv" "$vm_user@127.0.0.1:bin/cases.tsv"
if [[ "${1:-}" == --cleanup ]]; then
  vm_ssh "$steps cleanup"
  exit
fi

appimage=${1:-$(ls -t "$repo_root"/target/appimage/Qingjian-*.AppImage 2>/dev/null | head -1)}
[[ -f "$appimage" ]] || die '没有 AppImage：先跑 apps/linux/appimage/build.sh，或把文件路径传进来'
rm -rf "$out" && mkdir -p "$out/phases"
status=0
# phase 名称 远端命令：跑一段，打印并记下「通过 / 失败」行
phase() {
  local name=$1 result
  shift
  step "$name"
  result=$(vm_ssh "$*" 2>&1) || status=1
  printf '%s\n' "$result" | tee -a "$out/steps.log" | while IFS= read -r line; do
    case "$line" in 通过\ *) ok "${line#通过 }" ;; 失败\ *) fail "${line#失败 }" ;; esac
  done
  [[ "$result" != *失败* ]] || status=1
}
typing() {
  local name=$1 result
  step "$name"
  printf '环境\t应用\t结果\t按键\t期望\t实际\n' > "$out/phases/$2.tsv"
  result=$(vm_ssh "~/bin/insession ~/bin/e2e-app.sh x11 mousepad ~/bin/cases.tsv" 2>&1) || status=1
  vm_shot "$out/$2.png"
  e2e_collect "$2" mousepad "$result" "$out/phases/$2.tsv" || status=1
  # 失败时留下现场：曾有一次刚装好时 Server 记完「启动」就没了动静、插件连不上（2026-10-17，十来次里一次，没复现出来）
  if [[ "$result" == *FAIL* ]]; then
    vm_ssh 'echo "== 进程"; ps -eo pid,ppid,lstart,args | grep -E "[q]ingjian|[f]citx5"; echo "== socket"; ls -la "$XDG_RUNTIME_DIR"/qingjian.sock /run/user/$(id -u)/qingjian.sock 2>&1
      echo "== 挂载"; ls -d /tmp/.mount_* 2>&1; echo "== Server 日志"; tail -15 ~/.local/state/qingjian/logs/server.*.log
      echo "== Fcitx5 日志"; journalctl --user --since "-3min" --no-pager 2>/dev/null | grep -iE "fcitx|qingjian" | tail -15' > "$out/诊断-$2.txt" 2>&1 || true
    fail "现场留在 $out/诊断-$2.txt"
  fi
  vm_ssh 'pkill -x mousepad' || true
}

step "AppImage：$(basename "$appimage")（$(du -h "$appimage" | cut -f1)）"
if vm_ssh 'test -e ~/.local/share/qingjian/install-manifest.json || test -e ~/.local/bin/qingjian-linux-server'; then
  vm_ssh 'pkill -x qingjian-linux- || true; bash ~/qingjian/apps/linux/scripts/uninstall.sh >/dev/null' && ok '先卸掉了源码安装（用户数据保留）'
fi
vm_ssh 'mkdir -p ~/"Applications/青简 测试"'
vm_rsync "$appimage" "$vm_user@127.0.0.1:Applications/青简 测试/Qingjian.AppImage"
vm_ssh 'chmod +x ~/"Applications/青简 测试/Qingjian.AppImage"'

phase '安装（路径带空格与中文）' "$steps install"
typing '装好后打字（Mousepad）' 装好后
phase '从菜单打开设置' "$steps settings"
vm_shot "$out/设置.png"
vm_ssh "$steps close-settings" || true
step '重启虚拟机（验登录自启）'
"$here/vm.sh" reboot
phase '重启后' "$steps after-reboot"
typing '重启后不做任何操作直接打字' 重启后
phase '插件过期时 Server 自己同步' "$steps sync"
typing '同步后打字' 同步后
phase '重装（挪了文件、换了版本之后）' "$steps reinstall"
phase '卸载' "$steps uninstall"
step '再重启（卸载要干净）'
"$here/vm.sh" reboot
phase '卸载后重启' "$steps after-uninstall"

{
  printf '环境\t应用\t结果\t按键\t期望\t实际\n'
  # 只读 phases/ 下的：results.tsv 自己也在 $out 里，通配到它就会边读边写、无限长大（踩过，把磁盘写满了）
  for table in "$out"/phases/*.tsv; do tail -n +2 "$table"; done
} > "$out/results.tsv"
{
  e2e_summary "Linux AppImage 实测（Debian 13 + XFCE）：$(basename "$appimage")" "$out/results.tsv"
  echo
  echo '## 安装、自启、同步与卸载'
  echo
  grep -E '^(通过|失败) ' "$out/steps.log" | sed 's/^/- /'
} > "$out/summary.md"
step '汇总'
cat "$out/summary.md"
echo "截图与逐条结果：$out；之后要测源码安装，重跑 tools/verify/linux/check.sh"
exit "$status"
