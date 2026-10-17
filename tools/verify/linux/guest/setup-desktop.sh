#!/bin/sh
# //! 以 root 跑一次：LightDM 自动登录 XFCE、中文环境、输入法交给 Fcitx5、
# //! 源码装的青简 Server 随登录启动、测试用的输入框页面。用法：setup-desktop.sh 用户名
set -eu
user=$1
home=$(getent passwd "$user" | cut -d: -f6)
ln -sf /usr/share/zoneinfo/Asia/Shanghai /etc/localtime
sed -i 's/^# *\(zh_CN.UTF-8 UTF-8\)/\1/' /etc/locale.gen
locale-gen >/dev/null
mkdir -p /etc/lightdm/lightdm.conf.d
cat > /etc/lightdm/lightdm.conf.d/50-autologin.conf <<LDM
[Seat:*]
autologin-user=$user
autologin-user-timeout=0
autologin-session=xfce
LDM
systemctl set-default graphical.target >/dev/null
# 会话用中文界面（Debian 的 Xsession 读 ~/.xsessionrc，不读 ~/.xprofile）；
# 输入法变量由 im-config 按 ~/.xinputrc 设成 Fcitx5，Fcitx5 自己由 /etc/xdg/autostart 拉起
cat > "$home/.xsessionrc" <<'XS'
export LANG=zh_CN.UTF-8
XS
su - "$user" -c 'im-config -n fcitx5' >/dev/null
mkdir -p "$home/.config/autostart" "$home/.config/fcitx5" "$home/bin"
# 源码安装（check.sh）的 Server 随登录启动。不叫 qingjian-server.desktop：那个名字是 AppImage 的自启项，同名它会拒装
cat > "$home/.config/autostart/qingjian-server-source.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=青简 Server（源码安装）
Exec=sh -c 'test -x $home/.local/bin/qingjian-linux-server && exec $home/.local/bin/qingjian-linux-server'
NoDisplay=true
DESKTOP
# 输入法列表（加上青简）由 use-qingjian 在装好插件之后写：现在写了，Fcitx5 启动时认不出青简会把它丢掉
# 不锁屏、不息屏：light-locker 闲置后会锁屏、切到另一个 X（登录界面），测试会话那个 X 不在前台，按键全进不去
for name in light-locker xfce4-screensaver; do
  printf '[Desktop Entry]\nType=Application\nName=%s\nHidden=true\n' "$name" > "$home/.config/autostart/$name.desktop"
done
cat > "$home/.config/autostart/qingjian-no-blank.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=不息屏（测试机）
Exec=sh -c 'xset s off s noblank -dpms; xfconf-query -c xfce4-power-manager -p /xfce4-power-manager/dpms-enabled -n -t bool -s false'
NoDisplay=true
DESKTOP
# 测试用的网页：输入框铺满整页，点哪里都能聚焦
cat > "$home/input-test.html" <<'HTML'
<!doctype html><meta charset=utf-8><title>输入测试</title>
<textarea autofocus style="position:fixed;inset:0;width:100%;height:100%;font-size:20px"></textarea>
HTML
chown -R "$user:$user" "$home/.xsessionrc" "$home/.config" "$home/bin" "$home/input-test.html"
echo 桌面配置完成
