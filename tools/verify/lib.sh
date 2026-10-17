# shellcheck shell=bash
# //! tools/verify 各脚本共用的函数：仓库根、日志、虚拟机（FreeBSD / Linux）与 Mac 测试机的连接方式。只被 source，不单独执行。
# //! vm_* 函数缺省连 FreeBSD 虚拟机；Linux 那边的脚本 source 之后先 use_vm linux。
# 可调的环境变量：
#   QINGJIAN_VERIFY_OUT     结果目录（日志、截图），缺省 target/verify
#   QINGJIAN_FREEBSD_VM     FreeBSD 虚拟机目录（磁盘、密钥、QEMU 监视器），缺省 ~/vms/freebsd
#   QINGJIAN_LINUX_VM       Linux（Debian 13）虚拟机目录，缺省 ~/vms/debian13
#   QINGJIAN_VM_USER        虚拟机里的用户，缺省与本机同名
#   QINGJIAN_VM_SSH_PORT    FreeBSD 虚拟机 SSH 转发到本机的端口，缺省 2222
#   QINGJIAN_LINUX_SSH_PORT Linux 虚拟机的，缺省 2223（两台可以同时开）
#   QINGJIAN_MAC_HOST       Mac 测试机（user@host），没设就跳过 macOS 远程检查
#   QINGJIAN_MAC_DIR        Mac 上的同步目录，缺省 ~/work/qingjian-verify

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
verify_out=${QINGJIAN_VERIFY_OUT:-$repo_root/target/verify}
vm_user=${QINGJIAN_VM_USER:-$(id -un)}
mkdir -p "$verify_out"

step() { printf '\n\033[1;34m== %s\033[0m\n' "$*"; }
ok() { printf '\033[32m通过\033[0m %s\n' "$*"; }
fail() { printf '\033[31m失败\033[0m %s\n' "$*" >&2; }
die() { fail "$*"; exit 1; }

# use_vm freebsd|linux：后面的 vm_* 都连这台。
# ssh 选项里 StrictHostKeyChecking 关掉是因为虚拟机重建后主机密钥会变；
# 连接复用：QEMU 用户态网络的端口转发在短时间大量新建连接时偶尔卡在握手（banner exchange 超时），共用一条连接就少撞上
use_vm() {
  case "$1" in
    freebsd) vm_dir=${QINGJIAN_FREEBSD_VM:-$HOME/vms/freebsd} vm_port=${QINGJIAN_VM_SSH_PORT:-2222} ;;
    linux) vm_dir=${QINGJIAN_LINUX_VM:-$HOME/vms/debian13} vm_port=${QINGJIAN_LINUX_SSH_PORT:-2223} ;;
    *) die "不认识的虚拟机：$1" ;;
  esac
  vm_ssh_options=(-i "$vm_dir/id_ed25519" -p "$vm_port" -o StrictHostKeyChecking=no -o UserKnownHostsFile="$vm_dir/known_hosts"
    -o LogLevel=ERROR -o ConnectTimeout=20 -o ControlMaster=auto -o "ControlPath=$vm_dir/ssh-%r" -o ControlPersist=10m)
}
use_vm freebsd

# vm_ssh [root@] 命令…：缺省以 vm_user 登录。ssh 自己出错（退出码 255，多半是连接没建起来）重连再试，最多三次
vm_ssh() {
  local user=$vm_user attempt code
  if [[ "${1:-}" == *@ ]]; then user=${1%@}; shift; fi
  for attempt in 1 2 3; do
    # 写成 && || 的形式：调用方开着 set -e，失败的 ssh 不能直接把脚本带走，得先判断要不要重试
    ssh "${vm_ssh_options[@]}" "$user@127.0.0.1" "$@" && code=0 || code=$?
    ((code == 255)) || return "$code"
    printf '\033[33mssh 连不上虚拟机（第 %d 次），重试\033[0m\n' "$attempt" >&2
    ssh "${vm_ssh_options[@]}" -O exit "$user@127.0.0.1" 2>/dev/null
    sleep 3
  done
  return 255
}

vm_rsync() {
  rsync -a -e "ssh ${vm_ssh_options[*]}" "$@"
}

vm_running() { [[ -f "$vm_dir/qemu.pid" ]] && kill -0 "$(cat "$vm_dir/qemu.pid")" 2>/dev/null; }

# 等虚拟机里某个条件成立（远端命令返回 0），最多等 $2 秒
vm_wait() {
  local check=$1 limit=${2:-300} waited=0
  until vm_ssh "$check" >/dev/null 2>&1; do
    vm_running || die '虚拟机没在运行'
    ((waited >= limit)) && return 1
    sleep 3; ((waited += 3))
  done
}

# QEMU 监视器截屏：vm_shot 输出.png [x0 y0 x1 y1]
vm_shot() {
  local out=$1; shift
  echo "screendump $vm_dir/shot.ppm" | socat - "unix-connect:$vm_dir/monitor.sock" >/dev/null
  python3 -c '
import sys
from PIL import Image
image = Image.open(sys.argv[1])
box = [int(v) for v in sys.argv[3:7]]
(image.crop(box) if box else image).save(sys.argv[2])' "$vm_dir/shot.ppm" "$out" "$@"
}

# 图形端到端（guest/e2e-app.sh）的输出：一行一个用例，PASS/FAIL/FLAKY、按键、期望、实际，\x1f 分隔。
# e2e_collect 环境 应用 输出 results.tsv：逐条打印并追加到 results.tsv；一条结果都没有返回 1
e2e_collect() {
  local env=$1 app=$2 result=$3 table=$4 verdict keys expected actual
  while IFS=$'\x1f' read -r verdict keys expected actual; do
    [[ "$verdict" == PASS || "$verdict" == FAIL || "$verdict" == FLAKY ]] || continue
    printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$env" "$app" "$verdict" "$keys" "$expected" "$actual" >> "$table"
    case "$verdict" in
      PASS) ok "$keys → ${actual:-（空）}" ;;
      FLAKY) printf '\033[33m不稳定\033[0m %s：重试才过，%s\n' "$keys" "$actual" ;;
      *) fail "$keys：期望「$expected」，实际「$actual」" ;;
    esac
  done <<< "$result"
  [[ "$result" == *PASS* || "$result" == *FAIL* || "$result" == *FLAKY* ]] || { fail "$env · $app 没跑出结果：$result"; return 1; }
}

# e2e_summary 标题 results.tsv：按环境 × 应用汇总成 Markdown 表，失败与重试才过的单列
e2e_summary() {
  local title=$1 table=$2
  echo "# $title"
  echo
  echo "$(date '+%F %T')，$(git -C "$repo_root" rev-parse --short HEAD)$(git -C "$repo_root" diff --quiet || echo +)，用例 tools/verify/cases.tsv"
  echo
  echo '| 环境 | 应用 | 通过 | 重试才过 | 失败 |'
  echo '|---|---|---|---|---|'
  awk -F'\t' 'NR > 1 {k = $1 " | " $2; total[k]; if ($3 == "PASS") p[k]++; else if ($3 == "FLAKY") r[k]++; else f[k]++}
    END {for (k in total) printf "| %s | %d | %d | %d |\n", k, p[k], r[k], f[k]}' "$table" | sort
  if grep -q $'\tFAIL\t' "$table"; then
    echo; echo '## 失败的用例'; echo
    awk -F'\t' '$3 == "FAIL" {printf "- %s · %s：`%s` 期望「%s」，实际「%s」\n", $1, $2, $4, $5, $6}' "$table"
  fi
  if grep -q $'\tFLAKY\t' "$table"; then
    echo; echo '## 重试才过的用例（多半是机器忙时丢键，连着出现要查）'; echo
    awk -F'\t' '$3 == "FLAKY" {printf "- %s · %s：`%s` %s\n", $1, $2, $4, $6}' "$table"
  fi
}

# 列出与 main 相比改了的文件（含未提交的），用来决定跑哪些检查
changed_files() {
  local base
  base=$(git -C "$repo_root" merge-base HEAD "${QINGJIAN_VERIFY_BASE:-origin/main}" 2>/dev/null || echo HEAD)
  { git -C "$repo_root" diff --name-only "$base"; git -C "$repo_root" ls-files --others --exclude-standard; } | sort -u
}
