#!/usr/bin/env bash
# //! 仅删除本安装清单中的文件，保留配置、词库、学习数据。
set -euo pipefail
install_prefix="$HOME/.local"
while (($#)); do
  case "$1" in
    --prefix) install_prefix=${2:?--prefix 需要路径}; shift 2 ;;
    --help) echo '用法：uninstall.sh [--prefix 安装时的绝对目录]'; exit 0 ;;
    *) echo "未知参数：$1" >&2; exit 2 ;;
  esac
done
# 先停服务再删文件：单元文件被删之后 systemd 就不认这个名字了，停不掉的 Server 会一直占着 socket。
# **只停这一份装的**：systemd 用户单元按名字全局认，与 --prefix 无关。不核对的话，卸载一份临时测试装的
# 会把本机真装的那份也停掉、自启也关掉（踩过）。核对两件事：单元文件在这份的安装清单里，
# 且 systemd 实际加载的正是这个文件。
config_home="${XDG_CONFIG_HOME:-$HOME/.config}"
unit="$config_home/systemd/user/qingjian-server.service"
manifest="$install_prefix/share/qingjian/install-manifest.json"
if command -v systemctl >/dev/null 2>&1 \
  && [[ -f "$manifest" ]] && grep -qF "\"$unit\"" "$manifest" \
  && [[ "$(systemctl --user show -p FragmentPath --value qingjian-server.service 2>/dev/null)" == "$unit" ]]; then
  systemctl --user disable --now qingjian-server.service 2>/dev/null || true
fi
python3 "$(dirname -- "${BASH_SOURCE[0]}")/files.py" uninstall "$install_prefix"
command -v systemctl >/dev/null 2>&1 && systemctl --user daemon-reload || true
# IBus 的注册表缓存不跟 IBUS_COMPONENT_PATH 走（见 install.sh），一并删掉让它重建
rm -f "${XDG_CACHE_HOME:-$HOME/.cache}/ibus/bus/registry"
echo '卸载完成；用户数据已保留。fcitx5 重启一下、IBus 要重新登录一次（组件路径是会话启动时读的）才会忘掉这份青简。'
