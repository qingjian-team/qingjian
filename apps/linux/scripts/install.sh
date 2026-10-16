#!/usr/bin/env bash
# //! 用户目录安装：按这台机器上有哪个输入法框架装对应的前端（fcitx5 插件 / IBus 前端），Server 两边共用。
# //! 装哪几支由 --frontend 决定，缺省 auto = 能装的都装；不碰系统目录，也不碰用户数据。
set -euo pipefail
repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)
install_prefix="$HOME/.local"
build_profile=release
cmake_type=Release
sample=false
frontend=auto
enable_service=false
while (($#)); do
  case "$1" in
    --prefix) install_prefix=${2:?--prefix 需要路径}; shift 2 ;;
    --frontend) frontend=${2:?--frontend 需要 auto/fcitx5/ibus/both}; shift 2 ;;
    --debug) build_profile=debug; cmake_type=Debug; shift ;;
    --sample) sample=true; shift ;;
    --enable-service) enable_service=true; shift ;;
    --help)
      cat <<'USAGE'
用法：install.sh [选项]
  --prefix 目录        安装到哪（缺省 ~/.local）
  --frontend 值        auto（缺省，能装的都装）/ fcitx5 / ibus / both
  --debug              装 debug 构建
  --sample             只装样例词库，不要产品数据
  --enable-service     顺手 systemctl --user enable --now 把 Server 挂上开机自启
USAGE
      exit 0 ;;
    *) echo "未知参数：$1" >&2; exit 2 ;;
  esac
done
[[ "$install_prefix" = /* && "$install_prefix" != / ]] || { echo '需要非根绝对安装路径' >&2; exit 2; }
case "$frontend" in auto|fcitx5|ibus|both) ;; *) echo "--frontend 只认 auto / fcitx5 / ibus / both" >&2; exit 2 ;; esac

# fcitx5 那一支的插件是 C++，要编译器与 fcitx5 的开发包；缺了只能说清楚缺什么。
fcitx5_missing() {
  local missing=() tool module
  for tool in cmake c++ pkg-config; do
    command -v "$tool" >/dev/null 2>&1 || missing+=("$tool")
  done
  if command -v pkg-config >/dev/null 2>&1; then
    for module in Fcitx5Core Fcitx5Utils Fcitx5Config nlohmann_json; do
      pkg-config --exists "$module" || missing+=("$module")
    done
  fi
  echo "${missing[*]:-}"
}

# IBus 那一支是纯 Rust，编译不缺东西；装了没用才是问题，所以看的是机器上有没有 ibus。
ibus_missing() {
  command -v ibus-daemon >/dev/null 2>&1 || echo ibus-daemon
}

# 现在这个会话实际在用哪个框架。**只用来提示**，不决定装什么：
# 装完再切框架是常事，两支都装着不冲突（Server 按连接隔离会话）。
current_framework() {
  local hint="${XMODIFIERS:-}${GTK_IM_MODULE:-}${QT_IM_MODULE:-}"
  case "$hint" in
    *fcitx*) echo fcitx5 ;;
    *ibus*) echo ibus ;;
    *)
      # GNOME 会话里内建的就是 IBus，即使这几个变量都没设
      case "${XDG_CURRENT_DESKTOP:-}" in *GNOME*) echo ibus ;; *) echo 未知 ;; esac ;;
  esac
}

fcitx5_gap=$(fcitx5_missing)
ibus_gap=$(ibus_missing)
in_use=$(current_framework)
want_fcitx5=false
want_ibus=false
case "$frontend" in
  fcitx5) want_fcitx5=true ;;
  ibus) want_ibus=true ;;
  both) want_fcitx5=true; want_ibus=true ;;
  auto)
    [[ -z "$fcitx5_gap" ]] && want_fcitx5=true
    [[ -z "$ibus_gap" ]] && want_ibus=true ;;
esac
if [[ "$want_fcitx5" == true && -n "$fcitx5_gap" ]]; then
  echo "装不了 fcitx5 那一支，缺：$fcitx5_gap" >&2
  echo 'Debian / Ubuntu：sudo apt install cmake g++ pkg-config libfcitx5core-dev libfcitx5utils-dev libfcitx5config-dev nlohmann-json3-dev' >&2
  exit 1
fi
if [[ "$want_ibus" == true && -n "$ibus_gap" && "$frontend" != auto ]]; then
  echo "装不了 IBus 那一支，缺：$ibus_gap（Debian / Ubuntu：sudo apt install ibus）" >&2
  exit 1
fi
if [[ "$want_fcitx5" == false && "$want_ibus" == false ]]; then
  echo '两个框架都没找到：fcitx5 缺开发包、机器上也没有 ibus-daemon。' >&2
  echo '装上其中一个再来，或用 --frontend 明确指定要装哪一支。' >&2
  exit 1
fi
# auto 跳过某一支要说出来，不然「装好了」与「那一支根本没装」看起来一模一样。
if [[ "$frontend" == auto ]]; then
  [[ "$want_fcitx5" == true ]] || echo "跳过 fcitx5 那一支（缺：$fcitx5_gap）"
  [[ "$want_ibus" == true ]] || echo "跳过 IBus 那一支（缺：$ibus_gap）"
fi
# 最容易踩的一脚：装完一切正常，唯独正在用的那个框架什么都没装上。
if [[ "$in_use" == fcitx5 && "$want_fcitx5" == false ]]; then
  echo "注意：这个会话在用 fcitx5，但这次没装 fcitx5 那一支（缺：$fcitx5_gap）" >&2
fi
if [[ "$in_use" == ibus && "$want_ibus" == false ]]; then
  echo "注意：这个会话在用 IBus，但这次没装 IBus 那一支（缺：$ibus_gap）" >&2
fi

missing=()
for tool in cargo python3; do
  command -v "$tool" >/dev/null 2>&1 || missing+=("$tool")
done
command -v pkg-config >/dev/null 2>&1 && { pkg-config --exists openssl || missing+=(openssl); }
if ((${#missing[@]})); then
  echo "缺少依赖：${missing[*]}" >&2
  echo 'Debian / Ubuntu：sudo apt install python3 pkg-config libssl-dev；Rust 见 https://rustup.rs' >&2
  exit 1
fi

install_prefix=$(realpath -m -- "$install_prefix")
cargo_output=$(realpath -m -- "${CARGO_TARGET_DIR:-$repo_root/target}")
cargo_args=(build --manifest-path "$repo_root/Cargo.toml" --target-dir "$cargo_output" -p qingjian-linux-server -p qingjian-settings --locked)
[[ "$want_ibus" != true ]] || cargo_args+=(-p qingjian-linux-ibus)
[[ "$build_profile" != release ]] || cargo_args+=(--release)
cargo "${cargo_args[@]}"
built="$cargo_output/$build_profile"

files_args=(install "$install_prefix" --root "$repo_root" --server "$built/qingjian-linux-server" --settings "$built/qingjian-settings")
[[ "$sample" != true ]] || files_args+=(--sample)
staged=$(mktemp -d)
trap 'rm -rf -- "$staged"' EXIT

if [[ "$want_fcitx5" == true ]]; then
  cmake_output="$repo_root/target/fcitx5-install-$build_profile"
  cmake -S "$repo_root/apps/linux/fcitx5" -B "$cmake_output" "-DCMAKE_BUILD_TYPE=$cmake_type" -DBUILD_TESTING=OFF
  cmake --build "$cmake_output" --parallel "${CMAKE_BUILD_PARALLEL_LEVEL:-2}"
  files_args+=(--plugin "$cmake_output/qingjian.so")
fi
if [[ "$want_ibus" == true ]]; then
  # 组件 XML 里的 <exec> 是装好之后的绝对路径，所以现在就按 prefix 生成
  "$built/qingjian-linux-ibus" --ibus-xml "$install_prefix/bin/qingjian-linux-ibus" > "$staged/qingjian.xml"
  # **IBus 不读 ~/.local/share/ibus/component/**，只认 IBUS_COMPONENT_PATH（没设时只有系统目录），
  # 所以光把 XML 放进用户目录，真会话里根本看不到这个引擎（IBus 1.5.34 上验过）。
  # 用 environment.d 在会话启动时把用户目录加进去；**用户目录放最前面**：两个目录里有同名组件时
  # 以排在前面的为准（也验过），装过旧版 deb 的机器上新装的才会生效。
  data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
  # 只有 IBus 确实装在 /usr/local 下才换过去：装青简时 IBus 可能还没装，那时两处都不存在，
  # 退到 /usr/local 的话之后装上的系统组件（dconf 配置程序）就不在路径里，ibus-daemon 起不来（Arch 上踩过）
  system_components=/usr/share/ibus/component
  [[ ! -d "$system_components" && -d /usr/local/share/ibus/component ]] && system_components=/usr/local/share/ibus/component
  cat > "$staged/qingjian-ibus.conf" <<ENV
# 青简 IBus 前端（apps/linux/scripts/install.sh 生成，uninstall.sh 删除）。
# IBus 不读用户目录下的组件，只认这个变量；用户目录放最前面，同名组件以前面的为准。
IBUS_COMPONENT_PATH=$data_home/ibus/component:\${IBUS_COMPONENT_PATH:-$system_components}
ENV
  files_args+=(--ibus "$built/qingjian-linux-ibus" --ibus-xml "$staged/qingjian.xml" --ibus-env "$staged/qingjian-ibus.conf")
fi
if command -v systemctl >/dev/null 2>&1; then
  sed "s|@EXEC@|$install_prefix/bin/qingjian-linux-server|" \
    "$repo_root/apps/linux/scripts/qingjian-server.service.in" > "$staged/qingjian-server.service"
  files_args+=(--service "$staged/qingjian-server.service")
fi

python3 "$repo_root/apps/linux/scripts/files.py" "${files_args[@]}"
# ibus-daemon 的注册表缓存只看缓存里记着的目录变没变，IBUS_COMPONENT_PATH 换了它不管：
# 留着旧缓存，新加的目录（青简）可能看不到，缺了系统目录时 daemon 直接退出。删掉让它按新路径重建。
[[ "$want_ibus" != true ]] || rm -f "${XDG_CACHE_HOME:-$HOME/.cache}/ibus/bus/registry"

echo '已安装：'
[[ "$want_fcitx5" != true ]] || echo "  fcitx5 插件  $install_prefix/lib/fcitx5/qingjian.so"
[[ "$want_ibus" != true ]] || echo "  IBus 前端    $install_prefix/bin/qingjian-linux-ibus"
echo "  Server       $install_prefix/bin/qingjian-linux-server"
echo "  设置程序     $install_prefix/bin/qingjian-settings（应用菜单里的“青简设置”）"
# systemd 只从 ~/.config/systemd/user 找用户单元：配置目录被换到别处时（测试、临时安装）单元装在那里它看不见，
# 按名字 enable 反而会去开本机真装的那一份，所以这种情况不 enable、只提示
systemd_sees_unit=false
[[ "${XDG_CONFIG_HOME:-$HOME/.config}" == "$HOME/.config" ]] && systemd_sees_unit=true
if command -v systemctl >/dev/null 2>&1 && [[ "$systemd_sees_unit" == true ]]; then
  if [[ "$enable_service" == true ]]; then
    systemctl --user daemon-reload
    systemctl --user enable --now qingjian-server.service
    echo 'Server 已挂上开机自启（systemctl --user status qingjian-server）。'
  else
    echo "Server 现在还要手动起：$install_prefix/bin/qingjian-linux-server"
    echo '挂开机自启：systemctl --user daemon-reload && systemctl --user enable --now qingjian-server.service'
  fi
else
  [[ "$systemd_sees_unit" == true || "$enable_service" != true ]] \
    || echo 'XDG_CONFIG_HOME 不是 ~/.config，systemd 看不到这份单元，没有 enable（避免误开本机真装的那一份）。'
  echo "Server 要手动起：$install_prefix/bin/qingjian-linux-server"
fi
[[ "$want_fcitx5" != true ]] || echo 'fcitx5：重启 Fcitx5，在配置工具取消“仅显示当前语言”后添加“青简”。'
if [[ "$want_ibus" == true ]]; then
  echo 'IBus：**重新登录一次**（组件路径在会话启动时读），然后在“设置 → 键盘”里添加“青简”，或 ibus engine qingjian。'
  echo '      IBus 那一支只是前端，要 Server 起着才能打字；没加 --enable-service 的话记得按上面的命令挂自启。'
  if [[ -f /usr/share/ibus/component/qingjian.xml ]]; then
    echo '注意：系统里还装着另一份青简（/usr/share/ibus/component/qingjian.xml，多半是旧版 deb）。'
    echo '      重新登录后以这次装的为准；想回到那一份就跑 uninstall.sh。'
  fi
fi
