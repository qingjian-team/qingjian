#!/usr/bin/env bash
# //! FreeBSD 验证用虚拟机（QEMU + KVM）：从官方 cloud-init 镜像建机、开关机、登录、截屏。
# //! 建好后先跑 provision.sh 装编译依赖、桌面与应用；目录与端口见 lib.sh 的环境变量。
# 用法：vm.sh create | start [--gui] | stop | status | ssh [root@] [命令…] | shot 输出.png [x0 y0 x1 y1] | destroy
# --gui 在本机桌面开一个 QEMU 窗口显示虚拟机桌面（要本机有 DISPLAY / WAYLAND_DISPLAY）；不带就无界面跑，截屏照样能用。
set -euo pipefail
# shellcheck source=../lib.sh
source "$(dirname -- "${BASH_SOURCE[0]}")/../lib.sh"

release=${QINGJIAN_FREEBSD_RELEASE:-15.1}
image_url=https://download.freebsd.org/releases/VM-IMAGES/$release-RELEASE/amd64/Latest
image=FreeBSD-$release-RELEASE-amd64-BASIC-CLOUDINIT-ufs.qcow2

create() {
  [[ -f "$vm_dir/disk.qcow2" ]] && die "$vm_dir 下已经有虚拟机；要重建先 vm.sh destroy"
  for tool in qemu-system-x86_64 qemu-img xorriso socat; do command -v "$tool" >/dev/null || die "缺少 $tool（Ubuntu：apt install qemu-system-x86 qemu-utils xorriso socat）"; done
  [[ -r /dev/kvm && -w /dev/kvm ]] || die '/dev/kvm 不可读写：要在 kvm 组里'
  mkdir -p "$vm_dir" && cd "$vm_dir"
  step "下载并校验 FreeBSD $release 镜像"
  curl -fsSL -o CHECKSUM.SHA256 "$image_url/CHECKSUM.SHA256"
  [[ -f "$image.xz" ]] || curl -fL --retry 3 -o "$image.xz" "$image_url/$image.xz"
  grep "($image.xz)" CHECKSUM.SHA256 | awk -v f="$image.xz" '{print $NF "  " f}' | sha256sum -c - >/dev/null || die '镜像校验失败'
  xz -dkf "$image.xz" && mv -f "$image" disk.qcow2
  # Rust 编译产物、两套桌面加 Firefox 要三四十 G
  qemu-img resize -q disk.qcow2 80G
  [[ -f id_ed25519 ]] || ssh-keygen -q -t ed25519 -N '' -C qingjian-freebsd-vm -f id_ed25519
  step '生成 cloud-init 种子盘（用户、SSH 公钥、root 免密登录）'
  local key
  key=$(cat id_ed25519.pub)
  mkdir -p seed
  printf 'instance-id: qingjian-freebsd-1\nlocal-hostname: qingjian-bsd\n' > seed/meta-data
  cat > seed/user-data <<EOF
#cloud-config
users:
  - name: $vm_user
    groups: wheel
    shell: /bin/sh
    ssh_authorized_keys:
      - $key
runcmd:
  - mkdir -p /root/.ssh && chmod 700 /root/.ssh
  - echo '$key' >> /root/.ssh/authorized_keys && chmod 600 /root/.ssh/authorized_keys
  - sysrc sshd_enable=YES
  - echo 'PermitRootLogin prohibit-password' >> /etc/ssh/sshd_config
  - service sshd restart
EOF
  xorriso -as mkisofs -quiet -output seed.iso -volid cidata -joliet -rock seed/user-data seed/meta-data
  start
  step '等首次开机（会先自动打系统补丁，几分钟）'
  vm_wait true 900 || die '十五分钟内没等到 SSH，看 serial.log'
  vm_ssh 'freebsd-version'
  ok "虚拟机建好了，下一步：tools/verify/freebsd/provision.sh"
}

start() {
  vm_running && { echo '已经在运行'; return; }
  cd "$vm_dir"
  local display=(-display none)
  # FreeBSD 在 QEMU 里图形最稳的是标准 VGA + Xorg 的 vesa 驱动；usb-tablet 是绝对定位，鼠标进出窗口不用抓取
  # shellcheck disable=SC2054 # QEMU 的参数本身带逗号
  [[ "${1:-}" == --gui ]] && display=(-display gtk,zoom-to-fit=on)
  qemu-system-x86_64 -name qingjian-bsd -machine q35,accel=kvm -cpu host -smp "${QINGJIAN_VM_CPUS:-8}" -m "${QINGJIAN_VM_MEMORY:-12G}" \
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
  echo system_powerdown | socat - "unix-connect:$vm_dir/monitor.sock" >/dev/null 2>&1 || vm_ssh root@ 'shutdown -p now' || true
  for _ in $(seq 60); do kill -0 "$pid" 2>/dev/null || { echo '已关机'; return; }; sleep 1; done
  kill "$pid"; echo '一分钟没关掉，已强制结束'
}

case "${1:-}" in
  create) create ;;
  start) shift; start "$@" ;;
  stop) stop ;;
  status) if vm_running; then echo "运行中（pid $(cat "$vm_dir/qemu.pid")）"; vm_ssh 'freebsd-version; uptime' || true; else echo '没在运行'; fi ;;
  ssh) shift; vm_ssh "$@" ;;
  shot) shift; vm_shot "$@" ;;
  destroy) stop; rm -rf "$vm_dir"; echo "已删除 $vm_dir" ;;
  *) sed -n '2,5p' "$0" | sed 's/^# //'; exit 2 ;;
esac
