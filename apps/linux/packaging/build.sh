#!/usr/bin/env bash
# 打 Linux 发行包：deb（Debian 13 / Ubuntu 25.04+）、rpm（Fedora 42+）、flatpak（只有 IBus 前端）。
#
#   apps/linux/packaging/build.sh               三种都打
#   apps/linux/packaging/build.sh deb rpm       只打给出的几种
#   QINGJIAN_SAMPLE=1 apps/linux/packaging/build.sh   只带样例词库（没有产品数据时）
#
# 产物在 target/linux-pkg/out/。全部在容器里编，只要 docker（免 sudo）与 flatpak：
# - Rust 的 Server 与 IBus 前端在 Ubuntu 22.04 里编一次（glibc 2.35），三种包共用，老一点的发行版也跑得起来；
# - fcitx5 插件要对着目标发行版自己的 fcitx5 编（CMakeLists 要 5.1.8+），deb 在 Debian 13、rpm 在 Fedora 42 里编。
# 每种包里 Server 与数据一个包、fcitx5 插件一个包（IBus 前端没有额外依赖，跟 Server 同包）。
# 设计与限制见 docs/notes/linux-packaging.md；打完用 smoke/run.sh 在几个发行版的干净容器里装一遍自检。
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)
here="$root/apps/linux/packaging"
work="$root/target/linux-pkg"
out="$work/out"
formats=("$@")
((${#formats[@]})) || formats=(deb rpm flatpak)
for format in "${formats[@]}"; do
  case "$format" in deb|rpm|flatpak) ;; *) echo "只认 deb / rpm / flatpak：$format" >&2; exit 2 ;; esac
done
command -v docker >/dev/null || { echo '要 docker' >&2; exit 1; }

# 版本：Server 的 Cargo.toml 是 0.1.0-dev 这种；开发版接上 git 短哈希，发行格式各有各的写法
cargo_version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/apps/linux/server/Cargo.toml" | head -1)
upstream=${cargo_version%%-*}
git_rev=$(git -C "$root" rev-parse --short HEAD)
if [[ "$cargo_version" == *-dev ]]; then
  deb_version="${upstream}~dev+git${git_rev}"   # ~ 排在正式版之前
  rpm_release="0.dev.git${git_rev}"
else
  deb_version="$upstream"
  rpm_release=1
fi
echo "==> 版本 $cargo_version（$git_rev）"
mkdir -p "$out"

# 镜像只装编译工具，按内容打标签，改了才重建
image() {
  local name=$1 dockerfile=$2 tag
  tag="qingjian-pkg-$name:$(printf '%s' "$dockerfile" | sha256sum | cut -c1-12)"
  docker image inspect "$tag" >/dev/null 2>&1 || printf '%s' "$dockerfile" | docker build -q -t "$tag" - >/dev/null
  echo "$tag"
}
# in_container 镜像 [KEY=值…] 命令…：以本机用户身份跑，产物不会变成 root 的；仓库挂在 /src
in_container() {
  local tag=$1 env=()
  shift
  while [[ "${1:-}" == *=* ]]; do env+=(-e "$1"); shift; done
  docker run --rm --user "$(id -u):$(id -g)" -e HOME=/tmp "${env[@]}" -v "$root:/src" -w /src "$tag" "$@"
}

echo '==> Rust：Server 与 IBus 前端（Ubuntu 22.04）'
rust_image=$(image rust 'FROM ubuntu:22.04
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends build-essential pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*')
# 工具链与 crate 缓存直接用本机的（rustc 自己只要 glibc 2.17），仓库的 rust-toolchain.toml 照样生效
cargo_home=${CARGO_HOME:-$HOME/.cargo}
rustup_home=${RUSTUP_HOME:-$HOME/.rustup}
docker run --rm --user "$(id -u):$(id -g)" -e HOME=/tmp -e CARGO_HOME=/cargo -e RUSTUP_HOME=/rustup \
  -e CARGO_TARGET_DIR=/src/target/linux-pkg/cargo -v "$root:/src" -v "$cargo_home:/cargo" -v "$rustup_home:/rustup" -w /src \
  "$rust_image" /cargo/bin/cargo build --release --locked -p qingjian-linux-server -p qingjian-linux-ibus
bin="$work/cargo/release"

echo '==> 随包资源'
resources="$work/resources"
sample_flag=()
[[ "${QINGJIAN_SAMPLE:-}" != 1 ]] || sample_flag=(--sample)
python3 "$root/apps/linux/scripts/files.py" resources "$resources" --root "$root" "${sample_flag[@]}"

# Server 与数据那个包的公共部分摆进 $1（系统布局，前缀 /usr）
stage_core() {
  local dest=$1
  install -Dm755 "$bin/qingjian-linux-server" "$dest/usr/bin/qingjian-linux-server"
  install -Dm755 "$bin/qingjian-linux-ibus" "$dest/usr/bin/qingjian-linux-ibus"
  mkdir -p "$dest/usr/share/qingjian"
  cp -r "$resources" "$dest/usr/share/qingjian/resources"
  # 系统目录下的组件 IBus 直接认，不用 install.sh 那套 environment.d
  mkdir -p "$dest/usr/share/ibus/component"
  "$bin/qingjian-linux-ibus" --ibus-xml /usr/bin/qingjian-linux-ibus > "$dest/usr/share/ibus/component/qingjian.xml"
  mkdir -p "$dest/usr/lib/systemd/user"
  sed 's|@EXEC@|/usr/bin/qingjian-linux-server|' "$root/apps/linux/scripts/qingjian-server.service.in" \
    > "$dest/usr/lib/systemd/user/qingjian-server.service"
  # 托盘按名义尺寸挑图，目录名与像素必须一致（#547），与 files.py 装的同一套
  local size
  for size in 16 22 24 32 48 64 128 256; do
    install -Dm644 "$root/assets/icon/linux/hicolor/${size}x${size}/apps/qingjian.png" \
      "$dest/usr/share/icons/hicolor/${size}x${size}/apps/qingjian.png"
  done
}

build_deb() {
  echo '==> deb（Debian 13）'
  local tag stage="$work/deb" core fcitx
  tag=$(image deb 'FROM debian:trixie
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends cmake g++ make pkg-config dpkg-dev file libfcitx5core-dev libfcitx5utils-dev libfcitx5config-dev nlohmann-json3-dev && rm -rf /var/lib/apt/lists/*')
  rm -rf "$stage"
  core="$stage/qingjian"
  fcitx="$stage/qingjian-fcitx5"
  stage_core "$core"
  install -Dm644 "$root/LICENSE" "$core/usr/share/doc/qingjian/copyright"
  in_container "$tag" DEB_VERSION="$deb_version" bash /src/apps/linux/packaging/deb/assemble.sh
  rm -f "$out"/*.deb
  cp "$stage"/*.deb "$out/"
}

build_rpm() {
  echo '==> rpm（Fedora 42）'
  local tag stage="$work/rpm"
  tag=$(image rpm 'FROM fedora:42
RUN dnf install -y --setopt=install_weak_deps=False rpm-build cmake gcc-c++ make pkgconf-pkg-config fcitx5-devel json-devel systemd-rpm-macros && dnf clean all')
  rm -rf "$stage"
  stage_core "$stage/root"
  install -Dm644 "$root/LICENSE" "$stage/root/usr/share/licenses/qingjian/LICENSE"
  in_container "$tag" RPM_VERSION="$upstream" RPM_RELEASE="$rpm_release" bash /src/apps/linux/packaging/rpm/assemble.sh
  rm -f "$out"/*.rpm
  cp "$stage"/rpms/x86_64/*.rpm "$out/"
}

build_flatpak() {
  echo '==> flatpak'
  rm -f "$out"/*.flatpak
  bash "$here/flatpak/assemble.sh" "$bin" "$resources" "$out" "$deb_version"
}

for format in "${formats[@]}"; do "build_$format"; done
echo "==> 产物（$out）："
ls -lh "$out"
