#!/usr/bin/env bash
# //! AppImage 实测：每个发行版一个容器（真 Fcitx5、Mousepad、Xvfb、FUSE），跑 guest.sh；最后一张表。要 docker。
# 用法：test.sh [AppImage 文件] [发行版…]；缺省测 target/appimage/ 里最新的那个、下面列的全部发行版。
# 发行版：debian13 ubuntu2604 fedora arch（应能用）、ubuntu2404（Fcitx5 5.1.7 应拒装）、ubuntu2204（glibc 2.35 应拒装）、
# debian13-extract（不用 FUSE，APPIMAGE_EXTRACT_AND_RUN）
set -uo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
root=$(cd -- "$here/../../../.." && pwd)
appimage=${1:-$(ls -t "$root"/target/appimage/Qingjian-*.AppImage 2>/dev/null | head -1)}
[[ -f "$appimage" ]] || { echo '没有 AppImage，先跑 apps/linux/appimage/build.sh' >&2; exit 1; }
# docker -v 只认绝对路径，相对路径会被当成卷名
appimage=$(realpath -- "$appimage")
shift || true
distros=("$@")
((${#distros[@]})) || distros=(debian13 ubuntu2604 fedora arch ubuntu2404 ubuntu2204 debian13-extract)
logs="$root/target/appimage/test"
mkdir -p "$logs"

# 字体照各发行版桌面默认装的那几套：设置程序按路径找界面字体，容器里没字体就画不出来
deb_full='fcitx5 fcitx5-frontend-gtk3 mousepad xvfb xdotool xclip dbus-x11 dbus-daemon fuse3 procps desktop-file-utils ca-certificates fonts-noto-core fonts-noto-cjk'
deb_min='fcitx5 fuse3 procps'
dockerfile() {
  local base=$1 install=$2
  cat <<EOF
FROM $base
RUN $install
# Ubuntu 宿主的 AppArmor 按路径给 /usr/bin/fusermount3 套配置，容器里别家的 fusermount3 也会被套上：Debian 13 的
# 3.17 要 dac_override，被拦下就挂不上 AppImage（真机上不会：别家没有这份配置，Ubuntu 自己的版本与配置配套）。
# 挪个位置留符号链接，exec 时按真实路径匹配就套不上了；setuid 位跟着文件走
RUN p=\$(command -v fusermount3) && mv "\$p" /usr/local/lib/fusermount3-real && ln -s /usr/local/lib/fusermount3-real "\$p"
RUN useradd -m -s /bin/bash tester
USER tester
EOF
}
spec() {
  case "$1" in
    debian13|debian13-extract) dockerfile debian:trixie "apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends $deb_full" ;;
    ubuntu2604) dockerfile ubuntu:26.04 "apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends $deb_full" ;;
    ubuntu2404) dockerfile ubuntu:24.04 "apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends $deb_min" ;;
    ubuntu2204) dockerfile ubuntu:22.04 "apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends $deb_min" ;;
    fedora) dockerfile fedora:latest 'dnf install -y fcitx5 fcitx5-gtk mousepad google-noto-sans-vf-fonts google-noto-sans-cjk-vf-fonts xorg-x11-server-Xvfb xdotool xclip dbus-daemon dbus-tools fuse3 procps-ng desktop-file-utils findutils && dnf clean all' ;;
    arch) dockerfile archlinux:latest 'pacman -Syu --noconfirm fcitx5 fcitx5-gtk mousepad noto-fonts noto-fonts-cjk xorg-server-xvfb xdotool xclip dbus fuse3 procps-ng desktop-file-utils && pacman -Scc --noconfirm' ;;
    *) echo "不认识的发行版：$1" >&2; return 1 ;;
  esac
}
expectation() {
  case "$1" in
    ubuntu2404) echo fcitx5-too-old ;;
    ubuntu2204) echo glibc-too-old ;;
    *) echo ok ;;
  esac
}

declare -A result=()
for distro in "${distros[@]}"; do
  file=$(spec "$distro") || exit 2
  image="qingjian-appimage-test:${distro%-extract}-$(printf '%s' "$file" | sha256sum | cut -c1-12)"
  if ! docker image inspect "$image" >/dev/null 2>&1; then
    printf '%s\n' "$file" | docker build -q -t "$image" - >/dev/null || { result[$distro]='镜像建不出来'; continue; }
  fi
  mode=fuse
  [[ "$distro" == *-extract ]] && mode=extract
  # FUSE：容器要 /dev/fuse 与 SYS_ADMIN，Ubuntu 宿主的 AppArmor 也会拦挂载
  fuse=(--device /dev/fuse --cap-add SYS_ADMIN --security-opt apparmor=unconfined)
  [[ "$mode" == extract ]] && fuse=()
  echo "== $distro（$mode）"
  docker run --rm "${fuse[@]}" -v "$appimage:/appimage/Qingjian.AppImage:ro" -v "$here/guest.sh:/guest.sh:ro" "$image" \
    bash /guest.sh "$(expectation "$distro")" "$mode" 2>&1 | tee "$logs/$distro.log"
  if [[ ${PIPESTATUS[0]} == 0 ]]; then result[$distro]=通过; else result[$distro]=失败; fi
done

echo
echo "== $(basename "$appimage")"
status=0
for distro in "${distros[@]}"; do
  printf '  %-18s %s\n' "$distro" "${result[$distro]:-没跑}"
  [[ "${result[$distro]:-}" == 通过 ]] || status=1
done
echo "日志：$logs"
exit "$status"
