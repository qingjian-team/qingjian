//! 构建产物盘符门禁：Windows 上拒绝 C 盘。
//!
//! qingjian-format / qingjian-core / bamboo-slip 各有一份同样的 build.rs；构建脚本不能
//! 跨包引用仓库根文件，复制比给每个 crate 加 build-dependency 便宜。改一处要同步另外两处。
//!
//! 只看 OUT_DIR，不猜 CARGO_TARGET_DIR：Cargo 已经把最终 target 路径算进 OUT_DIR，
//! 换 target-dir 会重新跑构建脚本，所以这道门在第一次在该 target 编译时就会触发。

use std::path::{Component, Path, Prefix};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if !cfg!(windows) {
        return;
    }
    let Ok(out_dir) = std::env::var("OUT_DIR") else {
        return;
    };
    if is_c_drive(Path::new(&out_dir)) {
        panic!(
            "构建产物不能落在 C 盘：OUT_DIR={out_dir}。\
             请设置 CARGO_TARGET_DIR=E:/cargo-target/qingjian，\
             或在用户级 cargo config 里把 build.target-dir 指到非 C 盘。"
        );
    }
}

fn is_c_drive(path: &Path) -> bool {
    match path.components().next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => {
                letter.eq_ignore_ascii_case(&b'C')
            }
            _ => false,
        },
        _ => false,
    }
}
