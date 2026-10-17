#!/usr/bin/env bash
# //! Linux 验证用虚拟机（QEMU + KVM，Debian 13）：从官方 cloud-init 镜像建机、开关机、登录、截屏。
# //! 建好后先跑 provision.sh 装桌面、Fcitx5、编译依赖与测试应用；目录与端口见 lib.sh 的环境变量。
# 用法：vm.sh create | start [--gui] | stop | reboot | status | ssh [root@] [命令…] | shot 输出.png [x0 y0 x1 y1] | destroy
# --gui 在本机桌面开一个 QEMU 窗口显示虚拟机桌面；不带就无界面跑，截屏照样能用。
set -euo pipefail
here=$(dirname -- "${BASH_SOURCE[0]}")
# shellcheck source=../lib.sh
source "$here/../lib.sh"
use_vm linux

# 用 generic 不用 genericcloud：后者的精简内核没有图形驱动，换完整内核又缺 initramfs，重启起不来根分区
image_url=https://cloud.debian.org/images/cloud/trixie/latest
image=debian-13-generic-amd64.qcow2

create() {
  [[ -f "$vm_dir/disk.qcow2" ]] && die "$vm_dir 下已经有虚拟机；要重建先 vm.sh destroy"
  for tool in qemu-system-x86_64 qemu-img xorriso socat; do command -v "$tool" >/dev/null || die "缺少 $tool（Ubuntu：apt install qemu-system-x86 qemu-utils xorriso socat）"; done
  [[ -r /dev/kvm && -w /dev/kvm ]] || die '/dev/kvm 不可读写：要在 kvm 组里'
  mkdir -p "$vm_dir" && cd "$vm_dir"
  step '下载并校验 Debian 13 镜像'
  curl -fsSL -o SHA512SUMS "$image_url/SHA512SUMS"
  [[ -f "$image" ]] || curl -fL --retry 3 -o "$image" "$image_url/$image"
  grep " $image\$" SHA512SUMS | sha512sum -c - >/dev/null || die '镜像校验失败'
  cp "$image" disk.qcow2
  # Rust 编译产物、桌面与 Firefox 要二十来 G
  qemu-img resize -q disk.qcow2 40G
  [[ -f id_ed25519 ]] || ssh-keygen -q -t ed25519 -N '' -C qingjian-linux-vm -f id_ed25519
  step '生成 cloud-init 种子盘（用户、SSH 公钥、免密 sudo）'
  local key
  key=$(cat id_ed25519.pub)
  mkdir -p seed
  printf 'instance-id: qingjian-debian-%s\nlocal-hostname: qingjian-deb\n' "$(date +%s)" > seed/meta-data
  # 桌面登录密码只在这台本机虚拟机里用
  cat > seed/user-data <<EOF
#cloud-config
users:
  - name: $vm_user
    groups: [sudo]
    shell: /bin/bash
    sudo: ALL=(ALL) NOPASSWD:ALL
    lock_passwd: false
    plain_text_passwd: qingjian
    ssh_authorized_keys:
      - $key
  - name: root
    ssh_authorized_keys:
      - $key
disable_root: false
EOF
  xorriso -as mkisofs -quiet -output seed.iso -volid cidata -joliet -rock seed/user-data seed/meta-data
  start
  step '等首次开机'
  vm_wait 'cloud-init status --wait >/dev/null; true' 600 || die '十分钟内没等到 SSH，看 serial.log'
  vm_ssh 'cat /etc/debian_version'
  ok "虚拟机建好了，下一步：tools/verify/linux/provision.sh"
}

start() {
  vm_running && { echo '已经在运行'; return; }
  cd "$vm_dir"
  local display=(-display none)
  # shellcheck disable=SC2054 # QEMU 的参数本身带逗号
  [[ "${1:-}" == --gui ]] && display=(-display gtk,zoom-to-fit=on)
  qemu-system-x86_64 -name qingjian-linux -machine q35,accel=kvm -cpu host -smp "${QINGJIAN_VM_CPUS:-4}" -m "${QINGJIAN_VM_MEMORY:-6G}" \
    -drive file=disk.qcow2,if=virtio,format=qcow2,discard=unmap \
    -drive file=seed.iso,if=virtio,format=raw,readonly=on \
    -netdev "user,id=net0,hostfwd=tcp:127.0.0.1:$vm_port-:22" -device virtio-net-pci,netdev=net0 \
    -device virtio-rng-pci -vga std -device qemu-xhci -device usb-tablet -device usb-kbd \
    "${display[@]}" -serial file:serial.log -monitor unix:monitor.sock,server,nowait \
    -daemonize -pidfile qemu.pid
  echo "已启动（串口输出在 $vm_dir/serial.log）"
}

stop() {
  vm_running || { echo '没在运行'; return; }
  local pid
  pid=$(cat "$vm_dir/qemu.pid")
  echo system_powerdown | socat - "unix-connect:$vm_dir/monitor.sock" >/dev/null 2>&1 || vm_ssh root@ 'systemctl poweroff' || true
  for _ in $(seq 60); do kill -0 "$pid" 2>/dev/null || { echo '已关机'; return; }; sleep 1; done
  kill "$pid"; echo '一分钟没关掉，已强制结束'
}

# 重启并等自动登录的 XFCE 会话起来（登录自启这类事要靠它验）
reboot() {
  vm_ssh root@ 'systemctl reboot' || true
  ssh "${vm_ssh_options[@]}" -O exit "$vm_user@127.0.0.1" 2>/dev/null || true
  sleep 15
  vm_wait 'pgrep -x xfce4-session >/dev/null && pgrep -x fcitx5 >/dev/null' 300 || die '五分钟内没等到 XFCE 会话'
  # 自启项在会话起来之后才陆续跑
  sleep 10
  echo '已重启，XFCE 会话就绪'
}

case "${1:-}" in
  create) create ;;
  start) shift; start "$@" ;;
  stop) stop ;;
  reboot) reboot ;;
  status) if vm_running; then echo "运行中（pid $(cat "$vm_dir/qemu.pid")）"; vm_ssh 'cat /etc/debian_version; uptime' || true; else echo '没在运行'; fi ;;
  ssh) shift; vm_ssh "$@" ;;
  shot) shift; vm_shot "$@" ;;
  destroy) stop; rm -rf "$vm_dir"; echo "已删除 $vm_dir" ;;
  *) sed -n '2,5p' "$0" | sed 's/^# //'; exit 2 ;;
esac
