//! 青简 Linux 壳的 Rust 侧(staticlib):`host` 是业务，`bridge` 是给 C++ shim 的 C ABI,
//! `logging` 把 tracing 落到 `~/.local/state/qingjian/logs`。

mod bridge;
mod host;
mod logging;
