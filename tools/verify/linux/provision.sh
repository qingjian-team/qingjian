#!/usr/bin/env bash
# //! 给 vm.sh create 建好的 Linux 虚拟机装东西：packages.txt 里的包、guest/ 下的脚本、桌面配置、仓库要求的 Rust 版本。可重复跑。
# 用法：provision.sh
set -euo pipefail
here=$(dirname -- "${BASH_SOURCE[0]}")
# shellcheck source=../lib.sh
source "$here/../lib.sh"
use_vm linux
vm_running || "$here/vm.sh" start
vm_wait true 300 || die '等不到虚拟机的 SSH'
# root 也用同一把钥匙登录（vm.sh create 建的机器 cloud-init 已经放了；手工建的老机器这里补上）
vm_ssh 'sudo install -d -m 700 /root/.ssh && sudo install -m 600 ~/.ssh/authorized_keys /root/.ssh/authorized_keys'

step '装包（第一次要下载约 1.5 GB）'
packages=$(grep -v '^#' "$here/packages.txt" | tr '\n' ' ')
vm_ssh root@ "export DEBIAN_FRONTEND=noninteractive; apt-get update -q && apt-get install -y -q --no-install-recommends $packages" \
  > "$verify_out/linux-apt.log" 2>&1 || die "装包失败，完整输出：$verify_out/linux-apt.log"
ok "$(echo "$packages" | wc -w | tr -d ' ') 个包"

step '推送虚拟机里用的脚本到 ~/bin'
vm_ssh 'mkdir -p ~/bin'
vm_rsync "$here/guest/" "$here/../guest/" "$vm_user@127.0.0.1:bin/"

step '桌面配置（LightDM 自动登录 XFCE、Fcitx5、源码装的 Server 随登录启动）'
# 先停桌面：Fcitx5 退出时会把内存里的输入法列表写回 profile，开着写会被覆盖
vm_ssh root@ 'systemctl stop lightdm 2>/dev/null; for i in $(seq 30); do pgrep -x fcitx5 >/dev/null || break; sleep 1; done'
vm_ssh root@ "sh /home/$vm_user/bin/setup-desktop.sh $vm_user"

channel=$(sed -n 's/^channel = "\(.*\)"/\1/p' "$repo_root/rust-toolchain.toml")
step "Rust（rust-toolchain.toml 要求的 $channel）"
vm_ssh "[ -x ~/.cargo/bin/rustup ] || curl -sSf https://sh.rustup.rs | sh -s -- -y -q --profile minimal --default-toolchain $channel -c rustfmt,clippy >/dev/null; ~/.cargo/bin/rustup toolchain install -q --profile minimal $channel -c rustfmt,clippy >/dev/null; ~/.cargo/bin/rustc +$channel --version"

step '启动登录管理器，进 XFCE 会话'
vm_ssh root@ 'systemctl restart lightdm'
vm_wait 'pgrep -x xfce4-session >/dev/null && pgrep -x fcitx5 >/dev/null' 180 || die '三分钟内没等到 XFCE 会话'
ok '虚拟机准备好了，下一步：tools/verify/linux/check.sh（源码安装），或 tools/verify/linux/appimage.sh（AppImage）'
