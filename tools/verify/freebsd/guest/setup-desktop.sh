#!/bin/sh
# //! 以 root 跑一次：LightDM 自动登录、XFCE 中文环境、Fcitx5 与青简 Server 随登录启动、测试用的输入框页面。
# 用法：setup-desktop.sh 用户名
set -eu
user=$1
home=$(getent passwd "$user" | cut -d: -f6)
sysrc dbus_enable=YES lightdm_enable=YES >/dev/null
cp /usr/share/zoneinfo/Asia/Shanghai /etc/localtime
# usb-tablet 走 evdev，Xorg 装完提示的就是这个值
grep -q '^kern.evdev.rcpt_mask' /etc/sysctl.conf || echo kern.evdev.rcpt_mask=6 >> /etc/sysctl.conf
sysctl kern.evdev.rcpt_mask=6 >/dev/null
echo '%wheel ALL=(ALL:ALL) NOPASSWD: ALL' > /usr/local/etc/sudoers.d/wheel
# 桌面登录密码只在这台本机虚拟机里用
echo qingjian | pw usermod "$user" -h 0
mkdir -p /usr/local/etc/X11/xorg.conf.d /usr/local/etc/lightdm/lightdm.conf.d
cat > /usr/local/etc/X11/xorg.conf.d/20-qemu-vesa.conf <<'XORG'
Section "Device"
    Identifier "qemu-std-vga"
    Driver "vesa"
EndSection
XORG
cat > /usr/local/etc/lightdm/lightdm.conf.d/50-autologin.conf <<LDM
[Seat:*]
autologin-user=$user
autologin-user-timeout=0
user-session=xfce
LDM
pw groupadd autologin 2>/dev/null || true
pw groupmod autologin -m "$user"
# 会话：中文界面，输入模块走 Fcitx5；登录后自动起 fcitx5 与青简 Server
cat > "$home/.xprofile" <<'XP'
export LANG=zh_CN.UTF-8 LC_ALL=zh_CN.UTF-8
export GTK_IM_MODULE=fcitx QT_IM_MODULE=fcitx XMODIFIERS=@im=fcitx
XP
mkdir -p "$home/.config/autostart" "$home/.config/fcitx5" "$home/bin"
cp /usr/local/share/applications/org.fcitx.Fcitx5.desktop "$home/.config/autostart/"
cat > "$home/.config/autostart/qingjian-server.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=青简 Server
Exec=$home/.local/bin/qingjian-linux-server
NoDisplay=true
DESKTOP
# 输入法列表：英文键盘 + 青简。Fcitx5 退出时会把内存里的列表写回这个文件，所以只在它没运行时写
cat > "$home/.config/fcitx5/profile" <<'PROFILE'
[Groups/0]
Name=默认
Default Layout=us
DefaultIM=qingjian

[Groups/0/Items/0]
Name=keyboard-us
Layout=

[Groups/0/Items/1]
Name=qingjian
Layout=

[GroupOrder]
0=默认
PROFILE
# 测试用的网页：输入框铺满整页，点哪里都能聚焦
cat > "$home/input-test.html" <<'HTML'
<!doctype html><meta charset=utf-8><title>输入测试</title>
<textarea autofocus style="position:fixed;inset:0;width:100%;height:100%;font-size:20px"></textarea>
HTML
chown -R "$user:$user" "$home/.xprofile" "$home/.config" "$home/bin" "$home/input-test.html"
echo 桌面配置完成
