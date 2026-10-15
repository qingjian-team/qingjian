#!/usr/bin/env bash
# //! 预编译包的安装脚本（package.sh 放到包根目录的 install.sh）：解开产品数据，装到用户目录；Server 由用户手动启动。
set -euo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
install_prefix="$HOME/.local"
while (($#)); do
  case "$1" in
    --prefix) install_prefix=${2:?--prefix 需要路径}; shift 2 ;;
    --help) echo '用法：install.sh [--prefix 绝对用户目录]'; exit 0 ;;
    *) echo "未知参数：$1" >&2; exit 2 ;;
  esac
done
[[ "$install_prefix" = /* && "$install_prefix" != / ]] || { echo '需要非根绝对安装路径' >&2; exit 2; }
missing=()
for tool in python3 fcitx5; do
  command -v "$tool" >/dev/null 2>&1 || missing+=("$tool")
done
if ((${#missing[@]})); then
  echo "缺少依赖：${missing[*]}" >&2
  echo 'Debian / Ubuntu：sudo apt install python3 fcitx5 fcitx5-frontend-gtk3 fcitx5-frontend-gtk4 fcitx5-frontend-qt6 fcitx5-config-qt' >&2
  exit 1
fi
install_prefix=$(realpath -m -- "$install_prefix")
mkdir -p "$here/data"
# 数据包在 macOS 上打，带 LIBARCHIVE.xattr 扩展头，GNU tar 会逐个警告
tar --warning=no-unknown-keyword -xzf "$here/target/release-data/qingjian-data.tar.gz" -C "$here"
python3 "$here/files.py" install "$install_prefix" "$here" "$here/bin/qingjian-linux-server" "$here/bin/qingjian-settings" "$here/lib/qingjian.so" false
printf '已安装。手动启动：%s/bin/qingjian-linux-server\n重启 Fcitx5，在配置工具取消“仅显示当前语言”后添加“青简”。\n设置：应用菜单里的“青简设置”，或 %s/bin/qingjian-settings\n' "$install_prefix" "$install_prefix"
