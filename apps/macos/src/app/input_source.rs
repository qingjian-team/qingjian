//! 把 `.app` 注册成系统输入源、启用并切成当前输入源：`qingjian-macos --register`。
//!
//! 安装器（pkg 的 postinstall）以 root 跑，而输入源的注册与启用是每个用户自己的事，所以 postinstall 切到登录用户
//! 来调这个子命令；用户装完不用再去「系统设置 → 键盘 → 输入法」里手动添加。走 Carbon 的 Text Input Source Services，
//! 这套 C 接口至今没有 Cocoa 替代品。

use std::ffi::c_void;
use std::path::Path;
use std::ptr::NonNull;

use objc2_core_foundation::{CFArray, CFBoolean, CFDictionary, CFRetained, CFString, CFURL};
use objc2_foundation::{NSBundle, NSString};

/// TIS 的输入源句柄（不透明）。
#[repr(C)]
struct TISInputSource {
    _private: [u8; 0],
}

// HIToolbox 在 Carbon 伞框架里；CoreFoundation 由 objc2-core-foundation 链接
#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    /// 把一个输入法 bundle 登记到系统的输入源列表里（已登记的再登记无害）。
    fn TISRegisterInputSource(location: NonNull<CFURL>) -> i32;

    /// 按属性字典筛输入源；`include_all_installed` 为真时连没启用的也列出来。返回的数组归调用方释放。
    fn TISCreateInputSourceList(
        properties: *const c_void,
        include_all_installed: u8,
    ) -> *mut CFArray;

    /// 把输入源加进用户的输入法菜单。
    fn TISEnableInputSource(source: *const TISInputSource) -> i32;

    /// 把输入源切成当前输入源（装完就能打字，不用再去菜单里挑）。
    fn TISSelectInputSource(source: *const TISInputSource) -> i32;

    /// 读输入源的一个属性；返回值归系统，不释放。
    fn TISGetInputSourceProperty(
        source: *const TISInputSource,
        key: NonNull<CFString>,
    ) -> *const c_void;

    /// 属性键：输入源 ID（`Info.plist` 的 `TISInputSourceID`）。
    static kTISPropertyInputSourceID: NonNull<CFString>;

    /// 属性键：是否已启用（CFBoolean）。
    static kTISPropertyInputSourceIsEnabled: NonNull<CFString>;
}

/// 注册当前进程所在的 `.app`、启用并切成当前输入源。启用成功返回 `Ok(是否也切成了当前)`，失败带一句能打到安装日志里的说明。
pub fn register_main_bundle() -> Result<bool, String> {
    let bundle = NSBundle::mainBundle();
    let path = bundle.bundlePath().to_string();
    if !path.ends_with(".app") {
        return Err(format!("不是从 .app 里运行的：{path}"));
    }
    let source_id = bundle
        .objectForInfoDictionaryKey(&NSString::from_str("TISInputSourceID"))
        .and_then(|value| value.downcast::<NSString>().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| super::bundle::DEFAULT_IDENTIFIER.to_owned());
    register_and_enable(Path::new(&path), &source_id)
}

/// 注册 `app`，启用 ID 为 `source_id` 的输入源并切成当前。
///
/// 两个坑：刚换过 bundle 的头几秒系统还在重新扫描新包，这时启用的记录会被换掉、状态跟着丢（实测装完 3 秒内都这样）；
/// TIS 在进程内缓存输入源状态，本进程怎么重列表、跑 run loop 回读都是旧值，只有新起的进程看得到真实状态。
/// 所以每一轮都：注册 → 启用 → 起一个子进程（本程序带 `--finish-register`）在干净的缓存里回读并切成当前 →
/// 隔 [`CONFIRM`] 再起一次子进程确认没被重扫顶掉（顶掉了就整轮重来）；子进程说没启用就等 [`RETRY_INTERVAL`] 再来，最多等 [`ENABLE_TIMEOUT`]。
/// 返回是否也切成了当前输入源（切换失败不算错，用户还能从菜单里挑）。
pub fn register_and_enable(app: &Path, source_id: &str) -> Result<bool, String> {
    let url =
        CFURL::from_file_path(app).ok_or_else(|| format!("路径无法转成 URL：{}", app.display()))?;
    let exe = std::env::current_exe().map_err(|e| format!("找不到自己的可执行文件：{e}"))?;
    let started = std::time::Instant::now();
    loop {
        // SAFETY: url 是有效的 CFURL，函数只读它。
        let status = unsafe { TISRegisterInputSource(CFRetained::as_ptr(&url)) };
        if status != 0 {
            return Err(format!("TISRegisterInputSource 失败（{status}）"));
        }
        if let Some(sources) = list_sources(source_id).filter(|s| s.count() > 0) {
            for_each_source(&sources, |source| {
                // SAFETY: source 来自还活着的数组。
                let status = unsafe { TISEnableInputSource(source) };
                if status == 0 {
                    Ok(())
                } else {
                    Err(format!("TISEnableInputSource 失败（{status}）"))
                }
            })?;
            std::thread::sleep(SETTLE);
            let first = finish_in_child(&exe, source_id)?;
            if first != FINISH_NOT_ENABLED {
                std::thread::sleep(CONFIRM);
                let second = finish_in_child(&exe, source_id)?;
                match second {
                    FINISH_SELECTED => return Ok(true),
                    FINISH_ENABLED_ONLY => return Ok(false),
                    _ => eprintln!("输入源启用后又被系统重扫顶掉，重来"),
                }
            }
        }
        if started.elapsed() > ENABLE_TIMEOUT {
            return Err(format!(
                "输入源启用没有生效（等了 {} 秒）：{source_id}",
                ENABLE_TIMEOUT.as_secs()
            ));
        }
        std::thread::sleep(RETRY_INTERVAL);
    }
}

/// 起子进程跑 [`finish_register`]，返回它的退出码。
fn finish_in_child(exe: &Path, source_id: &str) -> Result<i32, String> {
    std::process::Command::new(exe)
        .arg(FINISH_FLAG)
        .arg(source_id)
        .status()
        .map_err(|e| format!("起不了子进程回读状态：{e}"))
        .map(|status| status.code().unwrap_or(FINISH_NOT_ENABLED))
}

/// `--register` 的收尾，在新进程里跑（见 [`register_and_enable`]）：ID 为 `source_id` 的输入源都已启用就切成当前。
/// 返回进程退出码：[`FINISH_SELECTED`] / [`FINISH_ENABLED_ONLY`] / [`FINISH_NOT_ENABLED`]。
pub fn finish_register(source_id: &str) -> i32 {
    let Some(sources) = list_sources(source_id).filter(|s| s.count() > 0) else {
        return FINISH_NOT_ENABLED;
    };
    let mut all_enabled = true;
    let mut selected = true;
    let _ = for_each_source(&sources, |source| {
        if !is_enabled(source) {
            all_enabled = false;
            return Ok(());
        }
        // SAFETY: source 来自还活着的数组。
        let status = unsafe { TISSelectInputSource(source) };
        if status != 0 {
            eprintln!("TISSelectInputSource 失败（{status}），输入源已启用但没切成当前");
            selected = false;
        }
        Ok(())
    });
    match (all_enabled, selected) {
        (false, _) => FINISH_NOT_ENABLED,
        (true, true) => FINISH_SELECTED,
        (true, false) => FINISH_ENABLED_ONLY,
    }
}

/// 子进程的命令行开关：`qingjian-macos --finish-register <输入源 ID>`。
pub const FINISH_FLAG: &str = "--finish-register";

/// 子进程退出码：已启用并切成当前。
pub const FINISH_SELECTED: i32 = 0;

/// 子进程退出码：没启用（要再等）。
pub const FINISH_NOT_ENABLED: i32 = 1;

/// 子进程退出码：已启用但切换失败。
pub const FINISH_ENABLED_ONLY: i32 = 2;

/// 回读输入源的启用状态。
fn is_enabled(source: *const TISInputSource) -> bool {
    // SAFETY: source 来自系统返回的数组且数组还活着；属性值是 CFBoolean，归系统所有，只读不释放。
    unsafe {
        let value = TISGetInputSourceProperty(source, kTISPropertyInputSourceIsEnabled);
        value
            .cast::<CFBoolean>()
            .as_ref()
            .is_some_and(CFBoolean::value)
    }
}

/// 对列表里的每个输入源调一次 `f`，第一个错误就返回。
fn for_each_source(
    sources: &CFArray,
    mut f: impl FnMut(*const TISInputSource) -> Result<(), String>,
) -> Result<(), String> {
    for index in 0..sources.count() {
        // SAFETY: index 在范围内；元素是 TISInputSourceRef，数组活着它就有效。
        let source = unsafe { sources.value_at_index(index) }.cast::<TISInputSource>();
        f(source)?;
    }
    Ok(())
}

/// 启用后等系统把新包扫描完再回读的时间。
const SETTLE: std::time::Duration = std::time::Duration::from_millis(1500);

/// 第一次确认成功后再等多久做第二次确认（系统重扫新包大约在装完 3–5 秒时把记录换掉）。
const CONFIRM: std::time::Duration = std::time::Duration::from_secs(3);

/// 两轮之间的间隔。
const RETRY_INTERVAL: std::time::Duration = std::time::Duration::from_millis(500);

/// 启用生效的等待上限。
const ENABLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// 列出 ID 为 `source_id` 的输入源（含未启用的）；系统没给数组时返回 `None`。
fn list_sources(source_id: &str) -> Option<CFRetained<CFArray>> {
    // SAFETY: kTISPropertyInputSourceID 是框架导出的常量字符串。
    let key: &CFString = unsafe { kTISPropertyInputSourceID.as_ref() };
    let value = CFString::from_str(source_id);
    let filter = CFDictionary::<CFString, CFString>::from_slices(&[key], &[&value]);
    // SAFETY: 字典有效；返回值按 Create 规则归调用方，用 from_raw 接管释放。
    unsafe {
        let raw =
            TISCreateInputSourceList(CFRetained::as_ptr(&filter).as_ptr().cast::<c_void>(), 1);
        NonNull::new(raw).map(|ptr| CFRetained::from_raw(ptr))
    }
}
