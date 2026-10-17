#!/bin/sh
# //! 以 root 跑：切换自动登录的桌面并重启 LightDM，等新会话里的 fcitx5 起来。用法：session.sh xfce | plasmax11
set -eu
case "$1" in
  xfce) leader=xfce4-session ;;
  plasmax11) leader=plasmashell ;;
  *) echo "不认识的会话：$1" >&2; exit 2 ;;
esac
conf=/usr/local/etc/lightdm/lightdm.conf.d/50-autologin.conf
if ! grep -qx "user-session=$1" "$conf" || ! pgrep -x "$leader" >/dev/null; then
  sed -i '' "s/^user-session=.*/user-session=$1/" "$conf"
  service lightdm restart >/dev/null 2>&1
fi
for _ in $(seq 120); do
  pgrep -x "$leader" >/dev/null && pgrep -x fcitx5 >/dev/null && { sleep 5; echo "会话 $1 就绪"; exit 0; }
  sleep 1
done
echo "两分钟内没等到会话 $1" >&2
exit 1
