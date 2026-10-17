#!/usr/bin/env bash
# //! 给 vm.sh create 建好的虚拟机装东西：packages.txt 里的包、guest/ 下的脚本、桌面配置、仓库要求的 Rust 版本。可重复跑。
# 用法：provision.sh
set -euo pipefail
here=$(dirname -- "${BASH_SOURCE[0]}")
# shellcheck source=../lib.sh
source "$here/../lib.sh"
vm_running || "$here/vm.sh" start

step '装包（第一次要下载约 3 GB）'
packages=$(grep -v '^#' "$here/packages.txt" | tr '\n' ' ')
vm_ssh root@ "env ASSUME_ALWAYS_YES=yes pkg install -y $packages" > "$verify_out/freebsd-pkg.log" 2>&1 \
  || die "装包失败，完整输出：$verify_out/freebsd-pkg.log"
ok "$(echo "$packages" | wc -w | tr -d ' ') 个包"

step '推送虚拟机里用的脚本到 ~/bin'
vm_ssh 'mkdir -p ~/bin'
vm_rsync "$here/guest/" "$here/../guest/" "$vm_user@127.0.0.1:bin/"

step '桌面配置（LightDM 自动登录 XFCE、Fcitx5 与青简 Server 随登录启动）'
# 先停桌面：Fcitx5 退出时会把内存里的输入法列表写回 profile，开着写会被覆盖
vm_ssh root@ 'service lightdm stop >/dev/null 2>&1; for i in $(seq 30); do pgrep -x fcitx5 >/dev/null || break; sleep 1; done'
vm_ssh root@ "sh /home/$vm_user/bin/setup-desktop.sh $vm_user"

step "Rust（rust-toolchain.toml 要求的 $(sed -n 's/^channel = "\(.*\)"/\1/p' "$repo_root/rust-toolchain.toml")）"
channel=$(sed -n 's/^channel = "\(.*\)"/\1/p' "$repo_root/rust-toolchain.toml")
vm_ssh "command -v rustup >/dev/null 2>&1 || [ -x ~/.cargo/bin/rustup ] || rustup-init -y -q --profile minimal --default-toolchain $channel -c rustfmt,clippy >/dev/null; ~/.cargo/bin/rustup toolchain install -q --profile minimal $channel -c rustfmt,clippy >/dev/null; ~/.cargo/bin/rustc +$channel --version"

step '重启登录管理器，进 XFCE 会话'
vm_ssh root@ "sh /home/$vm_user/bin/session.sh xfce"
ok '虚拟机准备好了，下一步：tools/verify/freebsd/check.sh'
