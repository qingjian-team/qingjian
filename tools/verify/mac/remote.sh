#!/usr/bin/env bash
# //! 在 Mac 测试机上跑：同步当前工作区 → 整个 workspace 的 clippy 与测试（与 Mac 上的 pre-commit / pre-push 同一范围）。
# //! --e2e 不跑检查（那是缺省模式的事，Mac 磁盘也放不下 debug 与 release 两份产物），同步后打包装上，用 osascript 往 TextEdit 发 cases.tsv 的按键、读回文本比对。
# 用法：QINGJIAN_MAC_HOST=user@host remote.sh [--e2e | --cases]；远端目录缺省 ~/work/qingjian-verify（QINGJIAN_MAC_DIR），不和手工同步的 ~/work/qingjian 混用
#   --e2e    打包安装再跑用例。/Library 下已有 pkg 版时只能装 pkg（要 sudo），打好包、打印安装命令后停下（退出码 3）；
#            ~/Library 下的开发版（或都没装）就直接 bundle.sh --install。新装的输入法要注销重登一次才出现在菜单里。
#   --cases  不同步不编译，只对已装的版本跑用例；先核对已装版本的短哈希与当前提交一致，免得测的是别的版本。
#            QINGJIAN_CASES=文件 换一份用例（格式同 tools/verify/cases.tsv）；QINGJIAN_MAC_DEBUG=1 让输入法打调试日志并取回；
#            QINGJIAN_MAC_EXPECT=短哈希 指定装着的应该是哪个提交。排查时用。
# 端到端会占用那台 Mac 的键盘（两边按键会穿插），跑之前先确认没人在用它。
set -euo pipefail
# shellcheck source=../lib.sh
source "$(dirname -- "${BASH_SOURCE[0]}")/../lib.sh"
host=${QINGJIAN_MAC_HOST:?要设 QINGJIAN_MAC_HOST=user@host}
mode=${1:-check}
case "$mode" in check|--e2e|--cases) ;; *) sed -n '2,9p' "$0" | sed 's/^# //'; exit 2 ;; esac
remote_dir=${QINGJIAN_MAC_DIR:-'~/work/qingjian-verify'}
# Xcode 许可没同意时 /usr/bin/git 与 cc 拒绝运行，改走 Command Line Tools；cargo 在 ~/.cargo/bin
remote_env='export DEVELOPER_DIR=/Library/Developer/CommandLineTools PATH="$HOME/.cargo/bin:$PATH"'

mac() { ssh -o BatchMode=yes -o ConnectTimeout=8 "$host" "$remote_env; $*"; }

step "连接 $host"
mac 'sw_vers -productVersion; uname -m' | paste -sd' ' || die "连不上 $host"

if [[ "$mode" != --cases ]]; then
  step '同步工作区（不碰那边的 target/ 与 data/；产品数据由 data-fetch.sh 取，git 历史经 bundle 传）'
  # .git 不直接拷：git worktree 的 .git 只是指向主仓库的文件，拷过去是坏的。改成把当前分支打成 git bundle，
  # 在那边建一个真仓库并指到同一个提交（只重置索引、不动同步过去的文件）。bundle.sh 的构建号是提交数，
  # 退成 1 的话 pkg 装不上去：安装器不拿构建号小的覆盖已装的版本（install.log 里是 Skipping component）
  rsync -a --delete --exclude /target --exclude /data --exclude /.git --exclude .flatpak-builder --exclude .codegraph \
    -e 'ssh -o BatchMode=yes' "$repo_root/" "$host:${remote_dir#\~/}/"
  bundle=$(mktemp --suffix=.bundle)
  git -C "$repo_root" bundle create -q "$bundle" HEAD
  scp -q -o BatchMode=yes "$bundle" "$host:/tmp/qingjian-verify.bundle"
  rm -f "$bundle"
  # 以前直接拷过 worktree 的 .git（一个指向本机路径的文件），留在那边会让 git 以为是坏仓库，先删掉
  mac "cd $remote_dir && { [ ! -f .git ] || rm .git; } && { [ -d .git ] || git init -q; } && git fetch -q /tmp/qingjian-verify.bundle HEAD && git reset -q FETCH_HEAD && rm -f /tmp/qingjian-verify.bundle && echo \"那边的仓库：\$(git rev-parse --short HEAD)，\$(git rev-list --count HEAD) 个提交\""
fi

if [[ "$mode" == check ]]; then
  step 'clippy（整个 workspace，与 Mac 上的 pre-commit 一致）'
  if mac "cd $remote_dir && cargo clippy --workspace --all-targets --locked -- -D warnings" > "$verify_out/mac-clippy.log" 2>&1; then
    ok 'clippy'
  else
    grep -E '^(error|warning)' "$verify_out/mac-clippy.log" | sort | uniq -c | head -20 >&2
    die "Mac 上 clippy 失败，完整输出：$verify_out/mac-clippy.log"
  fi

  step '测试（整个 workspace，与 Mac 上的 pre-push 一致）'
  if mac "cd $remote_dir && cargo test --workspace --locked" > "$verify_out/mac-test.log" 2>&1; then
    ok "$(awk '/^test result/ {p += $4; f += $6} END {print p " 个测试通过，" f " 个失败"}' "$verify_out/mac-test.log")"
  else
    grep -E 'FAILED|panicked|^error' "$verify_out/mac-test.log" | head -20 >&2
    die "Mac 上测试失败，完整输出：$verify_out/mac-test.log"
  fi
  exit 0
fi


app="/Library/Input Methods/Qingjian.app"
if [[ "$mode" == --e2e ]]; then
  if mac "test -d '$app'"; then
    step '已有 pkg 版（/Library）：打 pkg，装它要 sudo'
    mac "cd $remote_dir && tools/release/data-fetch.sh && apps/macos/scripts/bundle.sh --pkg" > "$verify_out/mac-bundle.log" 2>&1 \
      || die "打包失败，完整输出：$verify_out/mac-bundle.log"
    pkg=$(grep -oE '/[^ ]+\.pkg$' "$verify_out/mac-bundle.log" | tail -1)
    echo "在能输密码的终端里装（或在那台 Mac 上直接打开这个 pkg）："
    echo "  ssh -t $host 'sudo installer -pkg $pkg -target /'"
    echo "装好后：QINGJIAN_MAC_HOST=$host $0 --cases"
    exit 3
  fi
  step '打包并装到 ~/Library/Input Methods'
  mac "cd $remote_dir && tools/release/data-fetch.sh && apps/macos/scripts/bundle.sh --install" > "$verify_out/mac-bundle.log" 2>&1 \
    || die "打包失败，完整输出：$verify_out/mac-bundle.log"
  # $HOME 要在那台 Mac 上展开，不是本机的
  # shellcheck disable=SC2016
  app='$HOME/Library/Input Methods/Qingjian.app'
fi

step '核对装着的是这个提交'
# QINGJIAN_MAC_EXPECT=短哈希 指定要核对的版本：排查已装的某个版本、而本地又多了只改脚本的提交时用
expected=${QINGJIAN_MAC_EXPECT:-$(git -C "$repo_root" rev-parse --short HEAD)}
installed=$(mac "/usr/libexec/PlistBuddy -c 'Print CFBundleShortVersionString' '/Library/Input Methods/Qingjian.app/Contents/Info.plist' 2>/dev/null \
  || /usr/libexec/PlistBuddy -c 'Print CFBundleShortVersionString' ~/'Library/Input Methods/Qingjian.app/Contents/Info.plist'")
[[ "$installed" == *"$expected"* ]] || die "装着的是 $installed，不是当前提交 $expected（pkg 没装上？看那边的 /var/log/install.log）"
ok "$installed"

here=$(dirname -- "${BASH_SOURCE[0]}")
scp -q -o BatchMode=yes "$here/select-input.applescript" "$here/textedit.applescript" "$host:/tmp/"

# 那台 Mac 是有人日常用的：用例不能读它的配置（结果会随配置变），也不能把测试打的字学进它的学习数据。
# 先把整个数据目录挪开（输入法进程一并结束，免得它把内存里的状态写进新目录），输入法以全新状态（缺省配置、没有学习数据）跑；
# 跑完强制结束输入法（不让它把测试期间学到的写回去），删掉测试期间生成的目录再挪回来。中途出错也在退出时挪回。
# shellcheck disable=SC2088 # ~ 要在那台 Mac 上展开
data_dir='~/Library/Application\ Support/Qingjian'
backup="${data_dir}.e2e-backup"
restore_user_data() {
  mac "pkill -KILL -x qingjian-macos; if [ -d $backup ]; then rm -rf $data_dir && mv $backup $data_dir && echo '已恢复那台 Mac 的配置与学习数据'; fi" || \
    fail "没能恢复那台 Mac 的数据：手动把 $backup 挪回 $data_dir"
}
step '把那台 Mac 的配置与学习数据挪开，测试用全新状态'
# 先记下输入法日志现在有几行：后面等「预热完成」只看这之后新写的，免得认成上一个进程的
# shellcheck disable=SC2016 # 在那台 Mac 上展开
log_lines=$(mac 'wc -l < ~/Library/Logs/Qingjian/qingjian.log.$(date -u +%F) 2>/dev/null || echo 0' | tr -d ' ')
mac "[ ! -e $backup ] || { echo '上次的备份还在：$backup，先确认后手动挪回'; exit 1; }; pkill -KILL -x qingjian-macos; mv $data_dir $backup" \
  || die '挪不开数据目录，没有开始测试'
trap restore_user_data EXIT
# 排查用：QINGJIAN_MAC_DEBUG=1 时全新状态的配置只写一行 debug 日志级别，测完把这段日志取回 target/verify/mac-ime.log
if [[ -n "${QINGJIAN_MAC_DEBUG:-}" ]]; then
  mac "mkdir -p $data_dir && printf '[general]\nlog_level = \"debug\"\n' > $data_dir/config.toml"
fi

step '切到青简'
# 输入法进程被强制结束后菜单里会暂时没有它：--register 重新注册、启用，菜单要过几秒才刷新，等它出现再点
mac "\"$app/Contents/MacOS/qingjian-macos\" --register >/dev/null 2>&1
  for i in 1 2 3 4 5 6; do sleep 2; osascript /tmp/select-input.applescript 青简 2>/dev/null && exit 0; done; exit 1" \
  || die '输入法菜单里没有「青简」：在系统设置 → 键盘 → 输入法里添加，或注销重登一次'
# 输入法刚重启：词库与本地模型加载完之前敲的键会被吃掉，整句也出不来（含章·通变生成的候选），等日志里出现预热完成。
# 进程被结束后系统不一定马上重新拉起它（刚重新登录时就是），要有输入框拿到焦点才起：等不到就开一个 TextEdit 文档把它唤起来再等
# shellcheck disable=SC2016 # 在那台 Mac 上展开
wait_model='log=~/Library/Logs/Qingjian/qingjian.log.$(date -u +%F); for i in $(seq 30); do
    tail -n +'"$((log_lines + 1))"' "$log" 2>/dev/null | grep -q "本地模型已加载并预热" && exit 0; sleep 1; done; exit 1'
mac "$wait_model" || { mac "osascript /tmp/textedit.applescript WAIT" >/dev/null; mac "$wait_model"; } \
  || die '一分钟内没等到本地模型加载完（看那边的 ~/Library/Logs/Qingjian/）'

step 'TextEdit 端到端：发按键、读回文本（别碰那台 Mac 的键盘）'
status=0
while IFS=$'\t' read -r keys expected; do
  [[ -z "$keys" || "$keys" == \#* ]] && continue
  actual=$(mac "osascript /tmp/textedit.applescript '$keys'" < /dev/null) || actual='（osascript 出错）'
  if [[ "$actual" == "$expected" ]]; then ok "$keys → ${actual:-（空）}"; else fail "$keys：期望「$expected」，实际「$actual」"; status=1; fi
done < "${QINGJIAN_CASES:-$here/../cases.tsv}"
if [[ -n "${QINGJIAN_MAC_DEBUG:-}" ]]; then
  # shellcheck disable=SC2016 # 在那台 Mac 上展开
  mac 'tail -n +'"$((log_lines + 1))"' ~/Library/Logs/Qingjian/qingjian.log.$(date -u +%F)' > "$verify_out/mac-ime.log" || true
  echo "输入法日志：$verify_out/mac-ime.log"
fi
exit "$status"
