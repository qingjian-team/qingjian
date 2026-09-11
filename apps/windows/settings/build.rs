//! 编译时抓 git 构建标识（分支@短哈希 (日期)）塞进 `QINGJIAN_BUILD`，「关于」页显示；拿不到就不设。
//! 在 Windows 上编时把青简图标嵌进 exe（开始菜单 / 任务栏 / 搜索里显示的就是它）。

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../../../.git/HEAD");
    if let Some(build) = git_build() {
        println!("cargo:rustc-env=QINGJIAN_BUILD={build}");
    }
    embed_icon();
}

/// 图标资源要 `rc.exe`（MSVC）编，只在 Windows 宿主上做；失败只警告，别让编译挂掉。
#[cfg(windows)]
fn embed_icon() {
    const ICON: &str = "../tsf/resources/qingjian.ico";
    println!("cargo:rerun-if-changed={ICON}");
    if let Err(error) = winresource::WindowsResource::new().set_icon(ICON).compile() {
        println!("cargo:warning=嵌入设置程序图标失败: {error}");
    }
}

#[cfg(not(windows))]
fn embed_icon() {}

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
