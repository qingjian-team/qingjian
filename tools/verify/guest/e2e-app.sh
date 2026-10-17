#!/usr/bin/env bash
# //! 对一个应用逐条跑端到端用例：清空 → 剪贴板放哨兵 → 切青简 → 按记号发键 → 全选复制 → 读回比对。
# //! FreeBSD 与 Linux 虚拟机共用，经各自的 insession 以普通用户跑；输出一行一个用例：PASS/FAIL、按键、期望、实际，用 \x1f 分隔（制表符分隔时空的期望值会被 read 吞掉）。
# //! 应用跑完不关，留给宿主截屏。
# 用法：insession e2e-app.sh x11|wayland mousepad|featherpad|kate|firefox 用例文件
set -uo pipefail
mode=$1 app=$2 cases=$3
sentinel=__qingjian_empty__
delay=120
say() { printf '%s\x1f%s\x1f%s\x1f%s\n' "$@"; }

if [[ "$mode" == wayland ]]; then
  set -a; source /tmp/qingjian-nested.env; set +a
  # 原生 Wayland：输入走 text-input-v3，不要 GTK / Qt 的 fcitx 模块；xdotool 仍经外层 X11（DISPLAY）把按键送进嵌套窗口
  unset GTK_IM_MODULE QT_IM_MODULE XMODIFIERS
  export GDK_BACKEND=wayland QT_QPA_PLATFORM=wayland MOZ_ENABLE_WAYLAND=1
  # 嵌套 sway 经外层 X11 收键，机器忙时快了会丢键、乱序
  delay=200
  # wl-copy / xclip 都留在后台持有剪贴板；不把输出接走的话 ssh 会一直等它们，宿主那边就卡住
  copy_in() { printf %s "$1" | wl-copy >/dev/null 2>&1; }
  paste_out() { wl-paste -n 2>/dev/null; }
else
  copy_in() { printf %s "$1" | xclip -selection clipboard >/dev/null 2>&1; }
  paste_out() { xclip -o -selection clipboard 2>/dev/null; }
fi

case "$app" in
  mousepad) launch=(mousepad); class=mousepad; app_id=org.xfce.mousepad ;;
  featherpad) launch=(featherpad); class=featherpad; app_id=featherpad ;;
  kate) launch=(kate -n); class=kate; app_id=org.kde.kate ;;
  firefox)
    profile=/tmp/qingjian-firefox-$mode
    rm -rf "$profile" && mkdir -p "$profile"
    browser=firefox; command -v firefox >/dev/null || browser=firefox-esr
    launch=("$browser" --new-instance --profile "$profile" "file://$HOME/input-test.html"); class=firefox; app_id=firefox ;;
  *) echo "不认识的应用：$app" >&2; exit 2 ;;
esac

# 所有测试用的应用都先关掉，不只是这一个：上一个没清理干净的话（比如宿主那头 ssh 断了），
# 窗口叠在一起，点击落到别的窗口上，后面全都切不到青简。Kate 收到 SIGTERM 不退，两秒后补 SIGKILL
apps=(mousepad featherpad kate firefox firefox-esr)
for running in "${apps[@]}"; do pkill -x "$running" 2>/dev/null; done
sleep 2
for running in "${apps[@]}"; do pkill -KILL -x "$running" 2>/dev/null; done
# 被杀掉的 Mousepad 下次启动会弹「是否恢复数据」，焦点落在对话框上就没有输入框
gsettings set org.xfce.mousepad.preferences.file session-restore never 2>/dev/null
sleep 1
# 脱离 ssh 会话在后台起应用：FreeBSD 用 daemon(8)，Linux 用 setsid
if [[ "$(uname -s)" == FreeBSD ]]; then
  daemon -f -o "/tmp/qingjian-e2e-$mode-$app.log" "${launch[@]}"
else
  setsid "${launch[@]}" </dev/null >"/tmp/qingjian-e2e-$mode-$app.log" 2>&1 &
fi

# 等窗口出来并把键盘焦点给它（Firefox 第一次起新配置要二三十秒）
window=
for _ in $(seq 60); do
  if [[ "$mode" == wayland ]]; then
    swaymsg -t get_tree 2>/dev/null | grep -q "\"app_id\": \"$app_id\"" && { window=nested; break; }
  else
    window=$(xdotool search --onlyvisible --class "$class" 2>/dev/null | tail -1)
    [[ -n "$window" ]] && break
  fi
  sleep 1
done
[[ -n "$window" ]] || { say FAIL "（启动）" "" "$app 窗口一分钟内没出来"; exit 1; }
sleep 3
if [[ "$mode" == wayland ]]; then
  swaymsg "[app_id=\"$app_id\"] focus" >/dev/null
  # 嵌套 sway 本身是外层 X11 的一个窗口：先让它拿到 X 的键盘焦点，按键才进得去
  outer=$(xdotool search --onlyvisible --classname wlroots 2>/dev/null | head -1)
  [[ -n "$outer" ]] && xdotool windowactivate --sync "$outer"
  xdotool mousemove --window "${outer:-$(xdotool getactivewindow)}" 600 350 click 1
else
  xdotool windowactivate --sync "$window"
  eval "$(xdotool getwindowgeometry --shell "$window")"
  xdotool mousemove --window "$window" $((WIDTH / 2)) $((HEIGHT / 2)) click 1
fi
sleep 0.5
# Kate 打开的是欢迎页，新建一个文档才有输入框
[[ "$app" == kate ]] && { xdotool key ctrl+n; sleep 1.5; }

# 清空输入框并确认真的空了：应用可能恢复上次没存的文档，刚起来时还没加载完，清一次不一定生效
clear_input() {
  for _ in 1 2 3; do
    xdotool key Escape ctrl+a BackSpace; sleep 0.5
    copy_in "$sentinel"; sleep 0.3
    xdotool key ctrl+a ctrl+c; sleep 0.5
    # 读回哨兵或空都算空：Kate 对空文档按 Ctrl+C 会把剪贴板清掉，而不是不动
    local left
    left=$(paste_out)
    [[ "$left" == "$sentinel" || -z "$left" ]] && return 0
    sleep 1
  done
  return 1
}

activate() {
  for _ in 1 2 3; do
    fcitx5-remote -s qingjian 2>/dev/null
    sleep 0.3
    [[ "$(fcitx5-remote -n 2>/dev/null)" == qingjian ]] && return 0
    # Wayland 下 fcitx5-remote 管不到还没聚焦的上下文，退回 Fcitx5 自己的切换键
    xdotool key ctrl+space; sleep 0.5
    [[ "$(fcitx5-remote -n 2>/dev/null)" == qingjian ]] && return 0
  done
  return 1
}

# 跑一条用例，把读回的文本（或出错原因）写到 actual
run_case() {
  local keys=$1 token
  clear_input || { actual="清不空输入框：$(paste_out)"; return; }
  activate || { actual="切不到青简（当前 $(fcitx5-remote -n 2>/dev/null)）"; return; }
  for token in $keys; do
    case "$token" in
      SPACE) xdotool key space ;;
      RET) xdotool key Return ;;
      ESC) xdotool key Escape ;;
      BS) xdotool key BackSpace ;;
      WAIT) sleep 1 ;;
      # 停一下再按下一个键：整句的本地模型重排是异步的，真人打字也会停顿
      *) xdotool type --delay "$delay" "$token"; sleep 0.8 ;;
    esac
    sleep 0.3
  done
  sleep 0.8
  xdotool key ctrl+a ctrl+c; sleep 0.5
  actual=$(paste_out)
  [[ "$actual" == "$sentinel" ]] && actual=
}

# 失败的重试一次：重试才过的记 FLAKY（汇总里单列提醒，不判整轮失败），两次都错才是 FAIL
status=0
while IFS=$'\t' read -r keys expected; do
  [[ -z "$keys" || "$keys" == \#* ]] && continue
  run_case "$keys"
  if [[ "$actual" == "$expected" ]]; then say PASS "$keys" "$expected" "$actual"; continue; fi
  first=$actual
  run_case "$keys"
  if [[ "$actual" == "$expected" ]]; then say FLAKY "$keys" "$expected" "第一次是「$first」"; else say FAIL "$keys" "$expected" "$actual"; status=1; fi
done < "$cases"
exit "$status"
