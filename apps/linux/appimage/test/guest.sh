#!/usr/bin/env bash
# //! 在测试容器里以普通用户跑：从带空格与中文的路径装 AppImage → 检查装出的文件 → 用自启项里的命令起 Server →
# //! Xvfb + 真 Fcitx5 + Mousepad（GTK3）里打字、读剪贴板比对 → 设置程序截图 → 卸载。由 test.sh 调用。
# 用法：guest.sh ok|fcitx5-too-old|glibc-too-old [fuse|extract]
set -uo pipefail
expect=$1 mode=${2:-fuse}
status=0
pass() { printf '通过 %s\n' "$*"; }
fail() { printf '失败 %s\n' "$*"; status=1; }

dir="$HOME/Applications/青简 测试"
mkdir -p "$dir"
cp /appimage/Qingjian.AppImage "$dir/" && chmod 755 "$dir/Qingjian.AppImage"
app="$dir/Qingjian.AppImage"
export XDG_RUNTIME_DIR="/tmp/runtime-$(id -u)"
mkdir -p -m 700 "$XDG_RUNTIME_DIR"
# 不用 FUSE：AppImage 解到临时目录再跑（没有 /dev/fuse 的容器、沙箱里就是这样）
[[ "$mode" == extract ]] && export APPIMAGE_EXTRACT_AND_RUN=1
data="$HOME/.local/share" config="$HOME/.config"

if version=$("$app" --version 2>&1); then pass "--version：$version"; else fail "--version：$version"; fi
out=$("$app" install 2>&1) && code=0 || code=$?
case "$expect" in
  fcitx5-too-old)
    if [[ $code != 0 && "$out" == *"青简要 5.1.8"* ]]; then pass "Fcitx5 太旧时拒装：$out"; else fail "Fcitx5 太旧时应拒装（退出码 $code）：$out"; fi
    exit "$status" ;;
  glibc-too-old)
    if [[ $code != 0 && "$out" == *"glibc 2.41"* ]]; then pass "glibc 太旧时拒装：$out"; else fail "glibc 太旧时应拒装（退出码 $code）：$out"; fi
    exit "$status" ;;
esac
[[ $code == 0 ]] && pass "install" || { fail "install（退出码 $code）：$out"; exit 1; }

addon="$data/fcitx5/addon/qingjian.conf"
autostart="$config/autostart/qingjian-server.desktop"
desktop="$data/applications/qingjian-settings.desktop"
for file in "$addon" "$data/fcitx5/inputmethod/qingjian.conf" "$desktop" "$autostart" "$data/icons/hicolor/48x48/apps/qingjian.png"; do
  [[ -f "$file" ]] || fail "没装出 $file"
done
library=$(sed -n 's/^Library=//p' "$addon")
[[ -f "$library.so" ]] && pass "插件拷到 $library.so" || fail "addon 的 Library=$library 指的文件不在"
if command -v desktop-file-validate >/dev/null; then
  for file in "$desktop" "$autostart"; do
    if problems=$(desktop-file-validate "$file" 2>&1) && [[ -z "$problems" ]]; then pass "desktop-file-validate ${file##*/}"; else fail "desktop-file-validate ${file##*/}：$problems"; fi
  done
fi

server_exe() {
  local link exe
  for link in /proc/[0-9]*/exe; do
    exe=$(readlink "$link" 2>/dev/null) || continue
    [[ "$exe" == */usr/bin/qingjian-linux-server ]] && { printf '%s\n' "$exe"; return 0; }
  done
  return 1
}
wait_server() {
  for _ in $(seq 100); do
    [[ -S "$XDG_RUNTIME_DIR/qingjian.sock" ]] && server_exe >/dev/null && return 0
    sleep 0.2
  done
  return 1
}
wait_server && pass "install 顺手起了 Server：$(server_exe)" || fail "install 之后 Server 没起来"
# 自启项的 Exec 必须原样能跑（路径有空格与中文，引号转义错了登录时就起不来）：停掉，照 Exec 起一次
pkill -f '/usr/bin/qingjian-linux-server' ; sleep 1
exec_line=$(sed -n 's/^Exec=//p' "$autostart")
sh -c "$exec_line" </dev/null >/dev/null 2>&1 &
wait_server && pass "按自启项的 Exec 起得来：$exec_line" || fail "按自启项的 Exec 起不来：$exec_line"

# 真 Fcitx5 + GTK3 应用里打字
typing() {
  local cases=(
    $'nihao SPACE\t你好'
    $'nihao RET\tnihao'
    $'jintiantianqihenhao SPACE\t今天天气很好'
    $'firefoxliulanqi SPACE\tFirefox浏览器'
  )
  mkdir -p "$config/fcitx5"
  cat > "$config/fcitx5/profile" <<'EOF'
[Groups/0]
Name=Default
Default Layout=us
DefaultIM=qingjian

[Groups/0/Items/0]
Name=keyboard-us
Layout=

[Groups/0/Items/1]
Name=qingjian
Layout=

[GroupOrder]
0=Default
EOF
  fcitx5 -d --replace >/tmp/fcitx5.log 2>&1
  for _ in $(seq 50); do fcitx5-remote >/dev/null 2>&1 && break; sleep 0.2; done
  export GTK_IM_MODULE=fcitx XMODIFIERS=@im=fcitx
  mousepad >/tmp/mousepad.log 2>&1 &
  local window=
  for _ in $(seq 60); do
    window=$(xdotool search --onlyvisible --class mousepad 2>/dev/null | tail -1)
    [[ -n "$window" ]] && break
    sleep 0.5
  done
  [[ -n "$window" ]] || { fail "Mousepad 窗口没出来：$(tail -3 /tmp/mousepad.log)"; return; }
  sleep 2
  xdotool windowactivate --sync "$window" 2>/dev/null || xdotool windowfocus --sync "$window"
  local line keys expected token actual
  for line in "${cases[@]}"; do
    keys=${line%%$'\t'*} expected=${line#*$'\t'}
    xdotool key Escape ctrl+a BackSpace; sleep 0.5
    fcitx5-remote -s qingjian; sleep 0.3
    for token in $keys; do
      case "$token" in
        SPACE) xdotool key space ;;
        RET) xdotool key Return ;;
        *) xdotool type --delay 120 "$token"; sleep 0.8 ;;
      esac
      sleep 0.3
    done
    sleep 0.8
    printf '' | xclip -selection clipboard
    xdotool key ctrl+a ctrl+c; sleep 0.5
    actual=$(xclip -o -selection clipboard 2>/dev/null)
    if [[ "$actual" == "$expected" ]]; then pass "打字 $keys → $actual"; else fail "打字 $keys：期望「$expected」，读回「$actual」（Fcitx5 当前 $(fcitx5-remote -n 2>/dev/null)）"; fi
  done
  grep -i "qingjian" /tmp/fcitx5.log | grep -iE "fail|error|cannot" | head -3
}
Xvfb :99 -screen 0 1280x800x24 >/tmp/xvfb.log 2>&1 &
export DISPLAY=:99
sleep 1
export -f typing pass fail
export config
# 打字的结果经 stdout 回来；typing 在子 shell 里，失败数要靠输出里的「失败」行判
typing_out=$(dbus-run-session -- bash -c typing 2>&1)
printf '%s\n' "$typing_out"
grep -q '^失败' <<< "$typing_out" && status=1
grep -q '^通过 打字' <<< "$typing_out" || { fail "一条打字结果都没有"; }

# 只验设置程序能从 AppImage 起来、画得出页面；截图脚本走完没有是它自己的事（main 上在 Linux 会停在没有的页面上，#682 修）
shots=$(mktemp -d)
"$app" settings --config /tmp/no-such-config.toml --png "$shots" >/tmp/settings.log 2>&1 && settings_code=0 || settings_code=$?
count=$(find "$shots" -name '*.png' | wc -l)
if ((count > 0)); then
  pass "设置程序从 AppImage 起得来，截了 $count 张图"
  ((settings_code == 0)) || printf '备注 截图脚本没走完（退出码 %s）：%s\n' "$settings_code" "$(grep -m1 ERROR /tmp/settings.log)"
else
  fail "设置程序一张图都没画出来：$(tail -3 /tmp/settings.log)"
fi

# 重装一次（挪了文件、换了版本时就是这样）：上次装的图标仍要算自己的，否则卸载会把它们留下，之后的源码安装拒绝覆盖
out=$("$app" install 2>&1) && pass "重装" || fail "重装：$out"
out=$("$app" uninstall 2>&1) && pass "uninstall" || fail "uninstall：$out"
for file in "$addon" "$autostart" "$desktop" "$library.so" "$data/icons/hicolor/48x48/apps/qingjian.png"; do
  [[ ! -e "$file" ]] || fail "卸载后还在：$file"
done
server_exe >/dev/null && fail "卸载后 Server 还在跑：$(server_exe)" || pass "卸载后 Server 停了、文件删了"
[[ -d "$data/qingjian" || -f "$config/qingjian/config.toml" ]] && pass "用户数据保留" || fail "用户数据目录没了"
exit "$status"
