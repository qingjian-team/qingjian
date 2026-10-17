#!/usr/bin/env bash
# //! 打 Linux AppImage：target/appimage/Qingjian-<版本>-x86_64.AppImage（Server、设置程序、Fcitx5 插件、产品数据）。
# //! 在 Debian 13 容器里编（见同目录 Dockerfile），先跑 tools/release/data-fetch.sh；要 docker。装法与限制见 docs/notes/linux-appimage.md。
set -euo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
root=$(cd -- "$here/../../.." && pwd)
out="$root/target/appimage"
# AppImage 官方的 type2 runtime（静态链接，宿主不用装 libfuse2）。换版本时：下载 runtime-x86_64、.sig 与 signing-pubkey.asc，
# gpg --verify 过了（主密钥指纹 570C 77AC EA40 C0F1 B758 902C BF96 CCA5 6490 F695）再改这两行
runtime_tag=20251108
runtime_sha256=2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d

sha256() { sha256sum "$1" | cut -d' ' -f1; }

# 容器里：编、摆 AppDir、打 squashfs，最后接上 runtime
in_container() {
  local version=$1 target="$out/build" appdir="$out/AppDir" scratch
  cd "$root"
  cargo build --release --locked -p qingjian-linux-server -p qingjian-settings --target-dir "$target"
  cmake -S apps/linux/fcitx5 -B "$out/fcitx5" -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=OFF
  cmake --build "$out/fcitx5" --parallel "${CMAKE_BUILD_PARALLEL_LEVEL:-4}"
  rm -rf "$appdir"
  scratch=$(mktemp -d)
  # 与源码安装共用 files.py 的清单与产品数据校验：装进 AppDir/usr，XDG 目录指到 usr/share 与临时目录
  XDG_DATA_HOME="$appdir/usr/share" XDG_CONFIG_HOME="$scratch" python3 apps/linux/scripts/files.py install "$appdir/usr" "$root" \
    "$target/release/qingjian-linux-server" "$target/release/qingjian-settings" "$out/fcitx5/qingjian.so" false fcitx5
  rm -rf "$scratch" "$appdir/usr/share/qingjian/install-manifest.json" "$appdir/usr/share/applications"
  # files.py 按安装位置改写了 Library= / Exec=，AppImage 的位置要到用户机器上才知道：放回原样，由 AppRun install 再填
  install -m 644 apps/linux/fcitx5/data/addon/qingjian.conf "$appdir/usr/share/fcitx5/addon/qingjian.conf"
  install -m 644 apps/linux/fcitx5/data/inputmethod/qingjian.conf "$appdir/usr/share/fcitx5/inputmethod/qingjian.conf"
  install -m 755 "$here/AppRun" "$appdir/AppRun"
  sed "s/@VERSION@/$version/" "$here/qingjian.desktop" > "$appdir/qingjian.desktop"
  cp "$appdir/usr/share/icons/hicolor/256x256/apps/qingjian.png" "$appdir/qingjian.png"
  ln -s qingjian.png "$appdir/.DirIcon"
  printf '%s\n' "$version" > "$appdir/usr/share/qingjian/appimage-version"
  local name="Qingjian-$version-x86_64.AppImage"
  rm -f "$out/AppDir.squashfs" "$out/$name"
  mksquashfs "$appdir" "$out/AppDir.squashfs" -root-owned -noappend -comp zstd -Xcompression-level 19 -b 1M -quiet
  cat "$out/cache/runtime-$runtime_tag" "$out/AppDir.squashfs" > "$out/$name"
  chmod 755 "$out/$name"
  rm -f "$out/AppDir.squashfs"
  echo "已打包：$out/$name（$(du -h "$out/$name" | cut -f1)）"
}

if [[ "${1:-}" == --in-container ]]; then
  in_container "$2"
  exit
fi

command -v docker >/dev/null || { echo '要 docker' >&2; exit 1; }
cd "$root"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' apps/linux/server/Cargo.toml | head -1)
if [[ "$version" == *-dev ]]; then
  version+="-$(git rev-parse --short=7 HEAD)"
  git diff --quiet HEAD || version+='+'
fi
tools/release/data-fetch.sh --verify
mkdir -p "$out/cache" "$out/cargo-home"
runtime="$out/cache/runtime-$runtime_tag"
if [[ ! -f "$runtime" || "$(sha256 "$runtime")" != "$runtime_sha256" ]]; then
  curl -sSfL -o "$runtime.part" "https://github.com/AppImage/type2-runtime/releases/download/$runtime_tag/runtime-x86_64"
  [[ "$(sha256 "$runtime.part")" == "$runtime_sha256" ]] || { echo 'type2 runtime 校验值不符' >&2; exit 1; }
  mv "$runtime.part" "$runtime"
fi
image="qingjian-appimage-build:$(sha256 "$here/Dockerfile" | cut -c1-12)"
docker image inspect "$image" >/dev/null 2>&1 || docker build -t "$image" "$here"
# 以宿主用户跑，产物归宿主用户；registry 缓存放 target/appimage/cargo-home，第二次起不用重下依赖
docker run --rm --user "$(id -u):$(id -g)" -e HOME=/tmp -e CARGO_HOME=/cargo -e CMAKE_BUILD_PARALLEL_LEVEL \
  -v "$out/cargo-home:/cargo" -v "$root:$root" -w "$root" "$image" \
  bash "$here/build.sh" --in-container "$version"
