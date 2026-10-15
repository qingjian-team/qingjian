#!/usr/bin/env bash
# 把 build.sh 打好的 deb / rpm 拿到几个发行版的干净容器里装一遍并自检（install.sh），不碰本机：
#   apps/linux/packaging/smoke/run.sh                 全部发行版
#   apps/linux/packaging/smoke/run.sh fedora:42       只跑给出的镜像
# 每个发行版打印：装上的版本、systemd 自启登记、IBus 组件、插件缺不缺库、打 nihao 的候选、本地整句模型加载、卸载后是否清干净。
# 容器里没有图形会话，真会话里切到青简打字还是要人在机器上试；flatpak 要宿主机的 flatpak 与 IBus，不在这里验。
set -euo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
out=$(cd -- "$here/../../../.." && pwd)/target/linux-pkg/out
all=('debian:trixie apt deb' 'ubuntu:26.04 apt deb' 'fedora:42 dnf rpm' 'fedora:latest dnf rpm' 'opensuse/tumbleweed zypper rpm')
failed=0
for spec in "${all[@]}"; do
  read -r image manager format <<<"$spec"
  if (($#)) && [[ " $* " != *" $image "* ]]; then continue; fi
  ls "$out"/*."$format" >/dev/null 2>&1 || { echo "没有 $format 包，先跑 apps/linux/packaging/build.sh $format" >&2; exit 2; }
  pkgs=$(mktemp -d)
  cp "$out"/*."$format" "$pkgs/"
  echo "######## $image"
  docker run --rm -v "$pkgs:/pkgs:ro" -v "$here/install.sh:/install.sh:ro" "$image" bash /install.sh "$manager" || failed=1
  rm -rf "$pkgs"
done
exit "$failed"
