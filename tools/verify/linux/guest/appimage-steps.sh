#!/bin/bash
# //! Linux 虚拟机里 AppImage 实测的各个阶段，由宿主的 appimage.sh 经 insession 逐段调用；输出「通过 …」「失败 …」一行一条。
# 用法：appimage-steps.sh install|settings|after-reboot|sync|reinstall|uninstall|after-uninstall|cleanup
set -uo pipefail
app="$HOME/Applications/青简 测试/Qingjian.AppImage"
data="${XDG_DATA_HOME:-$HOME/.local/share}" config="${XDG_CONFIG_HOME:-$HOME/.config}"
status=0
pass() { printf '通过 %s\n' "$*"; }
fail() { printf '失败 %s\n' "$*"; status=1; }

# 来自 AppImage 的 Server：exe 在挂载点（/tmp/.mount_*）或解开的临时目录里，旁边有 AppRun
appimage_server() {
  local link exe
  for link in /proc/[0-9]*/exe; do
    exe=$(readlink "$link" 2>/dev/null) || continue
    [[ "$exe" == */usr/bin/qingjian-linux-server && -f "${exe%/usr/bin/qingjian-linux-server}/AppRun" ]] && { echo "$exe"; return 0; }
  done
  return 1
}
wait_server() { for _ in $(seq 40); do appimage_server >/dev/null && return 0; sleep 0.5; done; return 1; }
restart_fcitx5() { fcitx5 -r -d >/dev/null 2>&1; sleep 3; pgrep -x fcitx5 >/dev/null; }

case "$1" in
  install)
    out=$("$app" install 2>&1) && pass "install：${out%%$'\n'*}" || fail "install：$out"
    wait_server && pass "install 当场起了 Server：$(appimage_server)" || fail 'install 之后 Server 没起来'
    for file in "$data/fcitx5/addon/qingjian.conf" "$data/fcitx5/inputmethod/qingjian.conf" "$data/applications/qingjian-settings.desktop" "$config/autostart/qingjian-server.desktop"; do
      [[ -f "$file" ]] || fail "没装出 $file"
    done
    for file in "$data/applications/qingjian-settings.desktop" "$config/autostart/qingjian-server.desktop"; do
      problems=$(desktop-file-validate "$file" 2>&1) && [[ -z "$problems" ]] || fail "desktop-file-validate ${file##*/}：$problems"
    done
    # Fcitx5 只在启动时加载插件；列表里的青简要在插件装好之后放（先卸掉的源码安装会让它被丢掉）
    ~/bin/use-qingjian && pass 'Fcitx5 重启后加载了青简、切得过去' || fail 'Fcitx5 里切不到青简' ;;
  settings)
    pkill -x qingjian-settin; sleep 1
    # 照菜单项启动，与在应用菜单里点「青简设置」一样；第一次要挂载 AppImage、加载字体，等久一点
    setsid gio launch "$data/applications/qingjian-settings.desktop" </dev/null >/tmp/qingjian-settings.log 2>&1 &
    window=
    # xdotool 按中文标题匹配不上（--name 用的正则不认多字节），按 WM_CLASS 找，再核对标题
    for _ in $(seq 60); do window=$(xdotool search --onlyvisible --class qingjian-settings 2>/dev/null | head -1); [[ -n "$window" ]] && break; sleep 1; done
    if [[ -n "$window" && "$(xdotool getwindowname "$window")" == 青简设置 ]]; then
      pass '从「青简设置」菜单项打开了设置窗口'
    else
      fail "设置窗口一分钟没出来：$(tail -3 /tmp/qingjian-settings.log)"
    fi ;;
  close-settings)
    pkill -x qingjian-settin; exit 0 ;;
  after-reboot)
    wait_server && pass "登录后 Server 由自启项拉起：$(appimage_server)" || fail '登录后 Server 没起来'
    grep -qF "$app" "$config/autostart/qingjian-server.desktop" && pass '自启项指向 AppImage 文件' || fail '自启项不对' ;;
  sync)
    plugin="$data/qingjian-appimage/qingjian.so"
    before=$(sha256sum "$plugin" | cut -d' ' -f1)
    # 模拟「拷出去的是旧版本的插件」：换了新 AppImage 之后就是这样
    printf x >> "$plugin"
    pkill -x qingjian-linux-; sleep 1
    setsid "$app" server </dev/null >/tmp/qingjian-appimage-server.log 2>&1 &
    wait_server || fail '重起 Server 失败'
    sleep 1
    [[ "$(sha256sum "$plugin" | cut -d' ' -f1)" == "$before" ]] && pass "Server 启动时把插件同步回包里的版本（$(cat /tmp/qingjian-appimage-server.log)）" || fail '插件没同步'
    restart_fcitx5 || fail 'Fcitx5 重启失败' ;;
  reinstall)
    # 挪了文件、换了版本之后用户会再 install 一次：上次装的东西（含图标）都还要算自己的
    out=$("$app" install 2>&1) && pass "重装：${out%%$'\n'*}" || fail "重装：$out"
    wait_server && pass '重装后 Server 在' || fail '重装后 Server 没起来' ;;
  uninstall)
    pkill -x qingjian-settin
    out=$("$app" uninstall 2>&1) && pass "uninstall：${out%%$'\n'*}" || fail "uninstall：$out"
    sleep 1
    appimage_server >/dev/null && fail "卸载后 Server 还在：$(appimage_server)" || pass '卸载后 Server 停了'
    for file in "$data/fcitx5/addon/qingjian.conf" "$data/applications/qingjian-settings.desktop" "$config/autostart/qingjian-server.desktop" "$data/qingjian-appimage" "$data/icons/hicolor/48x48/apps/qingjian.png"; do
      [[ ! -e "$file" ]] || fail "卸载后还在：$file"
    done ;;
  after-uninstall)
    sleep 5
    appimage_server >/dev/null && fail '卸载后重启，Server 又起来了' || pass '卸载后重启，没有 Server'
    [[ -f "$config/qingjian/config.toml" ]] && pass '配置与学习数据保留' || fail '配置没了' ;;
  cleanup)
    if [[ -x "$app" ]]; then "$app" uninstall >/dev/null 2>&1; fi
    rm -rf "$data/qingjian-appimage" "$HOME/Applications/青简 测试"
    pass '清掉了 AppImage' ;;
  *) echo "不认识的阶段：$1" >&2; exit 2 ;;
esac
exit "$status"
