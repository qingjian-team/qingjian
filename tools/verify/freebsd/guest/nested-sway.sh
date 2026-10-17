#!/bin/sh
# //! 在当前 X11 桌面里起停一个嵌套的 sway（Wayland），里面由 sway 拉起 Fcitx5（input-method-v2），测原生 Wayland 应用。
# //! 虚拟机里没有 /dev/dri，整机 Wayland 会话起不来，只能嵌套；嵌套 KWin 不把按键交给输入法，所以用 sway（见 docs/notes/linux-fcitx5.md）。
# //! 单独开 dbus-run-session：和外层会话共用总线会抢名字。起来后把 WAYLAND_DISPLAY 与总线地址写进 /tmp/qingjian-nested.env。
# 用法（以普通用户、经 insession）：nested-sway.sh start | stop
set -eu
env_file=/tmp/qingjian-nested.env
case "$1" in
  start)
    "$0" stop >/dev/null 2>&1 || true
    config=/tmp/qingjian-nested-sway.conf
    cat > "$config" <<CONF
output * resolution 1200x700
default_border normal
exec sh -c 'printf "WAYLAND_DISPLAY=%s\nDBUS_SESSION_BUS_ADDRESS=%s\nSWAYSOCK=%s\n" "\$WAYLAND_DISPLAY" "\$DBUS_SESSION_BUS_ADDRESS" "\$SWAYSOCK" > $env_file.tmp && mv $env_file.tmp $env_file'
exec fcitx5 > /tmp/qingjian-nested-fcitx5.log 2>&1
CONF
    rm -f "$env_file"
    env -u WAYLAND_DISPLAY -u DBUS_SESSION_BUS_ADDRESS WLR_BACKENDS=x11 WLR_RENDERER=pixman WLR_X11_OUTPUTS=1 \
      daemon -f -o /tmp/qingjian-nested-sway.log dbus-run-session -- sway -c "$config"
    for _ in $(seq 30); do
      [ -f "$env_file" ] && grep -q 'Loaded addon waylandim' /tmp/qingjian-nested-fcitx5.log 2>/dev/null && { sleep 2; cat "$env_file"; exit 0; }
      sleep 1
    done
    echo '30 秒内嵌套 sway 没起来，看 /tmp/qingjian-nested-sway.log' >&2
    exit 1 ;;
  stop)
    pkill -x sway || true
    rm -f "$env_file" ;;
  *) echo "用法：$0 start | stop" >&2; exit 2 ;;
esac
