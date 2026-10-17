#!/usr/bin/env bash
# //! 提交 / 推送 / 开 PR 前的总入口：按与 main 相比改了的文件挑检查项，逐项跑，最后列出每项的结果与耗时。
# //! 各项的范围、适用场景与环境准备见 docs/notes/testing.md。
# 用法：run.sh [--all | --only 项,项 | --list]
#   项：local cross fcitx5-docker freebsd freebsd-e2e linux linux-e2e linux-appimage mac mac-e2e
#   --list 只打印这次会跑哪些、为什么；mac / mac-e2e 要设 QINGJIAN_MAC_HOST，freebsd* / linux* 要先建好虚拟机（freebsd/vm.sh、linux/vm.sh create）
set -uo pipefail
here=$(dirname -- "${BASH_SOURCE[0]}")
# shellcheck source=lib.sh
source "$here/lib.sh"

declare -A why=()
pick() { [[ -n "${why[$1]:-}" ]] || why[$1]=$2; }
mode=auto only=
case "${1:-}" in
  --all) mode=all ;;
  --only) mode=only; only=${2:?--only 要给项名} ;;
  --list) mode=list ;;
  '') ;;
  *) sed -n '2,7p' "$0" | sed 's/^# //'; exit 2 ;;
esac

if [[ "$mode" == only ]]; then
  for item in ${only//,/ }; do pick "$item" '手动指定'; done
elif [[ "$mode" == all ]]; then
  for item in local cross fcitx5-docker freebsd freebsd-e2e linux linux-e2e linux-appimage mac; do pick "$item" '--all'; done
else
  files=$(changed_files)
  pick local '每次都跑'
  grep -qE '^(crates/|apps/|tools/[^v]|Cargo\.(toml|lock)$|rust-toolchain)' <<< "$files" && pick cross '改了代码或依赖：别的平台可能编不过'
  grep -qE '^(apps/linux/|crates/qingjian-(core|platform)/|Cargo\.lock$)' <<< "$files" && pick fcitx5-docker '改了 Linux 前端或 Core：插件 ctest 要过'
  if grep -qE '^(apps/linux/|apps/settings/|crates/|Cargo\.(toml|lock)$|tools/verify/freebsd/)' <<< "$files"; then
    pick freebsd '改了 Linux / FreeBSD 共用的代码'
    grep -qE '^(apps/linux/(fcitx5|server)/|crates/qingjian-(core|platform)/|tools/verify/(cases\.tsv|freebsd/))' <<< "$files" \
      && pick freebsd-e2e '改了输入链路：图形端到端要过'
  fi
  if grep -qE '^(apps/linux/|apps/settings/|crates/|Cargo\.(toml|lock)$|tools/verify/linux/)' <<< "$files"; then
    pick linux '改了 Linux 前端或共用的代码：Debian 13 上源码安装、对着发行版的 Fcitx5 跑插件测试'
    grep -qE '^(apps/linux/(fcitx5|server)/|crates/qingjian-(core|platform)/|tools/verify/(cases\.tsv|guest/|linux/))' <<< "$files" \
      && pick linux-e2e '改了输入链路：Linux 真桌面里的图形端到端要过'
  fi
  grep -qE '^(apps/linux/appimage/|tools/verify/linux/(appimage\.sh|guest/appimage))' <<< "$files" \
    && pick linux-appimage '改了 AppImage：装、打字、登录自启、卸载整套要在真桌面里走一遍'
  grep -qE '^(apps/macos/|crates/|Cargo\.(toml|lock)$)' <<< "$files" && pick mac '改了 macOS 壳或共享 crate'
fi
# 环境没准备好的项跳过，并说明怎么准备
declare -A skip=()
for item in "${!why[@]}"; do
  case "$item" in
    freebsd|freebsd-e2e) [[ -f "$vm_dir/disk.qcow2" ]] || skip[$item]="没有 FreeBSD 虚拟机（$vm_dir）：tools/verify/freebsd/vm.sh create && provision.sh" ;;
    linux|linux-e2e|linux-appimage)
      [[ -f "${QINGJIAN_LINUX_VM:-$HOME/vms/debian13}/disk.qcow2" ]] \
        || skip[$item]="没有 Linux 虚拟机（${QINGJIAN_LINUX_VM:-$HOME/vms/debian13}）：tools/verify/linux/vm.sh create && provision.sh"
      [[ "$item" != linux-appimage || -x "$repo_root/apps/linux/appimage/build.sh" ]] || skip[$item]='这个分支没有 apps/linux/appimage' ;;
    mac|mac-e2e) [[ -n "${QINGJIAN_MAC_HOST:-}" ]] || skip[$item]='没设 QINGJIAN_MAC_HOST=user@host' ;;
    fcitx5-docker) command -v docker >/dev/null || skip[$item]='没有 docker' ;;
  esac
done

order=(local cross fcitx5-docker freebsd freebsd-e2e linux linux-e2e linux-appimage mac mac-e2e)
step '这次跑'
for item in "${order[@]}"; do
  [[ -n "${why[$item]:-}" ]] || continue
  if [[ -n "${skip[$item]:-}" ]]; then printf '  %-14s 跳过：%s\n' "$item" "${skip[$item]}"; else printf '  %-14s %s\n' "$item" "${why[$item]}"; fi
done
[[ "$mode" == list ]] && exit 0

declare -A result=() seconds=()
for item in "${order[@]}"; do
  [[ -n "${why[$item]:-}" && -z "${skip[$item]:-}" ]] || continue
  start=$SECONDS
  case "$item" in
    local) "$here/local.sh" ;;
    cross) "$here/cross.sh" ;;
    fcitx5-docker) "$here/fcitx5-docker.sh" ;;
    freebsd) (vm_running || "$here/freebsd/vm.sh" start) && "$here/freebsd/check.sh" ;;
    # 同一轮里 freebsd 已经装过并通过，就直接跑图形测试
    freebsd-e2e) { [[ "${result[freebsd]:-}" == 通过 ]] || "$here/freebsd/check.sh" --skip-tests; } && "$here/freebsd/e2e.sh" ;;
    linux) "$here/linux/check.sh" ;;
    linux-e2e) { [[ "${result[linux]:-}" == 通过 ]] || "$here/linux/check.sh" --skip-tests; } && "$here/linux/e2e.sh" ;;
    # 放在 Linux 那两项之后：它会先卸掉虚拟机里的源码安装
    linux-appimage) "$repo_root/apps/linux/appimage/build.sh" && "$here/linux/appimage.sh" ;;
    mac) "$here/mac/remote.sh" ;;
    mac-e2e) "$here/mac/remote.sh" --e2e ;;
  esac
  if (($? == 0)); then result[$item]=通过; else result[$item]=失败; fi
  seconds[$item]=$((SECONDS - start))
done

step '结果'
status=0
for item in "${order[@]}"; do
  [[ -n "${why[$item]:-}" ]] || continue
  if [[ -n "${skip[$item]:-}" ]]; then printf '  %-14s 跳过\n' "$item"; continue; fi
  printf '  %-14s %s（%d 秒）\n' "$item" "${result[$item]}" "${seconds[$item]}"
  [[ "${result[$item]}" == 通过 ]] || status=1
done
echo "日志与截图：$verify_out"
exit "$status"
