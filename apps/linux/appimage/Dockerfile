# //! AppImage 的构建环境：Fcitx5 ≥ 5.1.8（插件的 API 下限）的发行版里 glibc 最老的一个，编出来的程序在更新的系统上都能跑。
# //! Debian 13：Fcitx5 5.1.12、glibc 2.41。build.sh 按本文件的摘要给镜像打标签，改了这里会自动重建。
FROM debian:trixie
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl build-essential cmake pkg-config python3 git \
      libssl-dev libfcitx5core-dev libfcitx5utils-dev libfcitx5config-dev nlohmann-json3-dev squashfs-tools \
 && rm -rf /var/lib/apt/lists/*
# 工具链装在 /opt，容器以宿主用户身份跑时也能用；registry 缓存另由 build.sh 挂进来
ENV RUSTUP_HOME=/opt/rustup CARGO_HOME=/opt/cargo PATH=/opt/cargo/bin:$PATH
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain 1.96.0 --component rustfmt,clippy --no-modify-path \
 && chmod -R a+rX /opt/rustup /opt/cargo
