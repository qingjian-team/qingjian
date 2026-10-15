#!/usr/bin/env bash
# 在宿主机上跑（build.sh 调）：把编好的二进制与资源摆进 staging/，flatpak-builder 出仓库，再导出单文件 .flatpak。
#   assemble.sh <二进制目录> <资源目录> <产物目录> <版本>
set -euo pipefail
bin=$1 resources=$2 out=$3 version=$4
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
app=app.qingjian.Qingjian
work=$(dirname "$resources")/flatpak
runtime_version=25.08

command -v flatpak >/dev/null || { echo '没有 flatpak（Ubuntu：sudo apt install flatpak）' >&2; exit 1; }
# flatpak-builder 可以是本机命令，也可以是装成 flatpak 的 org.flatpak.Builder
if command -v flatpak-builder >/dev/null; then
  builder=(flatpak-builder)
elif flatpak info org.flatpak.Builder >/dev/null 2>&1; then
  builder=(flatpak run org.flatpak.Builder)
else
  echo '没有 flatpak-builder：flatpak install --user flathub org.flatpak.Builder' >&2
  exit 1
fi
for ref in "org.freedesktop.Platform//$runtime_version" "org.freedesktop.Sdk//$runtime_version"; do
  flatpak info "$ref" >/dev/null 2>&1 || flatpak install --user -y --noninteractive flathub "$ref"
done

rm -rf "$work"
staging="$work/staging"
install -Dm755 -t "$staging/bin" "$bin/qingjian-linux-server" "$bin/qingjian-linux-ibus" \
  "$here/qingjian-server" "$here/qingjian-ibus" "$here/qingjian-host-setup"
strip --strip-unneeded "$staging/bin/qingjian-linux-server" "$staging/bin/qingjian-linux-ibus"
mkdir -p "$staging/share/qingjian"
cp -r "$resources" "$staging/share/qingjian/resources"
# 组件 XML 的 <exec> 是宿主机上的 flatpak run；flatpak 的绝对路径各机器不同，登记时再换掉 @FLATPAK@
"$bin/qingjian-linux-ibus" --ibus-xml "@FLATPAK@ run --command=qingjian-ibus $app" > "$staging/share/qingjian/ibus-component.xml"
cp "$here/$app.yml" "$work/"

"${builder[@]}" --force-clean --disable-rofiles-fuse --state-dir="$work/state" --repo="$work/repo" "$work/build" "$work/$app.yml"
flatpak build-bundle "$work/repo" "$out/qingjian_${version}_x86_64.flatpak" "$app"
