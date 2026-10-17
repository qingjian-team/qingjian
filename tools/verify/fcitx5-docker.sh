#!/usr/bin/env bash
# //! 在 Ubuntu 容器里编 Fcitx5 插件并跑它的 ctest（48 个，含对真 Server 的测试），本机不用装 Fcitx5 开发包。
# //! Server 用本机编出的 target/debug/qingjian-linux-server，所以容器的 glibc 不能比本机旧；缺省镜像与 CI 无关，取 Ubuntu 26.04。
# 用法：fcitx5-docker.sh [镜像]，例如 fcitx5-docker.sh ubuntu:24.04 对齐 CI 的 Fcitx5 版本
set -euo pipefail
# shellcheck source=lib.sh
source "$(dirname -- "${BASH_SOURCE[0]}")/lib.sh"
cd "$repo_root"
base=${1:-ubuntu:26.04}
image="qingjian-fcitx5-test:${base#*:}"
command -v docker >/dev/null || die '要先装 docker'

if ! docker image inspect "$image" >/dev/null 2>&1; then
  step "准备容器镜像 $image（只在第一次）"
  docker build -q -t "$image" - >/dev/null <<DOCKERFILE
FROM $base
RUN apt-get update -qq && DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends \
    cmake g++ make pkg-config libfcitx5core-dev libfcitx5config-dev libfcitx5utils-dev nlohmann-json3-dev fcitx5 \
  && rm -rf /var/lib/apt/lists/*
DOCKERFILE
fi

step '编 Rust Server（插件测试要拉起真 Server）'
cargo build -q --locked -p qingjian-linux-server

step "在 $image 里编插件并跑 ctest"
log=$verify_out/fcitx5-docker.log
if docker run --rm -v "$repo_root:$repo_root:ro" -w /tmp "$image" bash -c "
  set -e
  cmake -S '$repo_root/apps/linux/fcitx5' -B /tmp/build -DCMAKE_BUILD_TYPE=Debug -DBUILD_TESTING=ON \
    -DQINGJIAN_SERVER_EXECUTABLE='$repo_root/target/debug/qingjian-linux-server' >/dev/null
  cmake --build /tmp/build --parallel \$(nproc)
  cd /tmp/build && env -u DBUS_SESSION_BUS_ADDRESS ctest --output-on-failure -j4
" > "$log" 2>&1; then
  ok "$(grep -E 'tests passed' "$log")"
else
  grep -E 'error|tests passed|\*\*\*|Assertion' "$log" | head -20 >&2
  die "插件测试失败，完整输出：$log"
fi
