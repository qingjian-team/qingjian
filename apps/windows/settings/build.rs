//! 编译时抓 git 构建标识（分支@短哈希 (日期)，工作区有改动短哈希后加 +，与 macOS 的 bundle.sh 一致）塞进 `QINGJIAN_BUILD`，「关于」页显示；拿不到就不设。
//! 在 Windows 上编时把青简图标嵌进 exe（开始菜单 / 任务栏 / 搜索里显示的就是它）。

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../../../.git/HEAD");
    if let Some(build) = git_build() {
        println!("cargo:rustc-env=QINGJIAN_BUILD={build}");
    }
    embed_icon();
    stage_windows_runtime();
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

/// 自包含部署 Windows App Runtime，并让 exe 在 Windows 10 上也能加载（定位与取舍见 docs\notes\windows-win10.md）。
///
/// 两件事都得做：
/// 1. **自带运行时**：Reactor 的框架依赖引导调的是 Windows 11 才有的 AppModel API
///    （`TryCreatePackageDependency` / `AddPackageDependency`，`api-ms-win-appmodel-runtime-l1-1-5.dll`），
///    Windows 10 上没有这两个函数，也就没法把机器上装的框架包加进进程包图；改成自包含部署后 UI 只用
///    exe 旁边这份运行时，`windows-reactor-setup` 负责铺文件并按自包含标记嵌清单。
/// 2. **延迟加载那两个 API**：它们是早期绑定，函数名会进 exe 的导入表（IAT），Windows 10 在**加载期**就报
///    「无法定位程序输入点 TryCreatePackageDependency」，根本进不到自包含分支。延迟加载后 IAT 里没有它们，
///    Windows 10 能正常启动；而自包含部署下这两个函数不会被调用。
#[cfg(windows)]
fn stage_windows_runtime() {
    // 宿主是 Windows 但目标不是（交叉编译到别的平台）时不出 Windows 产物，跳过。
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    windows_reactor_setup::as_self_contained();
    println!("cargo:rustc-link-arg-bins=/DELAYLOAD:api-ms-win-appmodel-runtime-l1-1-5.dll");
    println!("cargo:rustc-link-arg-bins=delayimp.lib");
}

#[cfg(not(windows))]
fn stage_windows_runtime() {}

fn git_build() -> Option<String> {
    let branch = git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
    let mut hash = git(&["rev-parse", "--short", "HEAD"])?;
    if git(&["status", "--porcelain"]).is_some() {
        hash.push('+');
    }
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
