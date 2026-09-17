cmake_minimum_required(VERSION 3.16)
project(qingjian-fcitx5 CXX)

set(CMAKE_CXX_STANDARD 20)   # fcitx5 5.1.x 头文件用 std::ranges/std::span,20 起步
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(Fcitx5Core REQUIRED)
# Rust staticlib 经 qingjian-platform → qingjian-predict 传递拖入 async-openai/reqwest,
# 引用一批 OpenSSL 符号（云联想用，Linux 壳其实不调）。显式链上 libssl/libcrypto,
# 让 .so 用 DT_NEEDED 老实声明依赖，任何装了 OpenSSL 3 的机器都能稳定加载——
# 而不是靠宿主进程碰巧预载了 openssl。
# （根治是给 predict 的网络依赖加 feature 开关、让只读配置的消费方不背这套栈，见上游 PR 清单。）
find_package(OpenSSL REQUIRED)

# Rust 侧 staticlib：先 cargo build -p qingjian-linux-host --release,
# 路径可用 -DQINGJIAN_RUST_LIB=... 覆盖（默认取仓库 target/release）。
if(NOT DEFINED QINGJIAN_RUST_LIB)
    set(QINGJIAN_RUST_LIB
        "${CMAKE_CURRENT_SOURCE_DIR}/../../../target/release/libqingjian_linux_host.a")
endif()
if(NOT EXISTS "${QINGJIAN_RUST_LIB}")
    message(FATAL_ERROR "Rust staticlib 不在:${QINGJIAN_RUST_LIB}(先 cargo build -p qingjian-linux-host --release)")
endif()

# 产物 libqingjian.so（保留 lib 前缀，与本机 libpinyin.so 等现役插件同惯例）
add_library(qingjian MODULE addon.cpp)
target_link_libraries(qingjian
    Fcitx5::Core
    "${QINGJIAN_RUST_LIB}"
    OpenSSL::SSL OpenSSL::Crypto
    pthread dl m)
