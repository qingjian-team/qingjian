//! 编译时抓取 git 构建标识（分支 @ 短哈希（日期）），塞进 `QINGJIAN_BUILD` 供「关于」页显示。
//! 拿不到 git（不在仓库里 / 没装 git）就不设，界面显示「本地构建」。

use std::process::Command;

fn main() {
    // 切分支或新提交时重跑（.git 在仓库根，相对本包目录往上三层）。
    println!("cargo:rerun-if-changed=../../../.git/HEAD");
    if let Some(build) = git_build() {
        println!("cargo:rustc-env=QINGJIAN_BUILD={build}");
    }
}

/// `windows@1a2b3c4 (2026-09-10)`。任一步拿不到就整体放弃。
fn git_build() -> Option<String> {
    let branch = git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
    let hash = git(&["rev-parse", "--short", "HEAD"])?;
    let date = git(&[
        "show",
        "-s",
        "--format=%cd",
        "--date=format:%Y-%m-%d",
        "HEAD",
    ])?;
    Some(format!("{branch}@{hash} ({date})"))
}

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    (!text.is_empty()).then_some(text)
}
