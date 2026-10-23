//! UI Automation 只读工作线程：不设焦点、不模拟按键、不调用任何控件写接口。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender;

use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation8, IUIAutomation2, IUIAutomationTextPattern, UIA_TextPatternId,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

use super::job::ReadResult;

// 提供方即使没有遵守超时，也最多占住一个线程，不因反复按快捷键累积工作线程。
static BUSY: AtomicBool = AtomicBool::new(false);

pub(super) fn spawn(
    process: u32,
    window: u64,
    cancelled: Arc<AtomicBool>,
    sender: SyncSender<ReadResult>,
) {
    if BUSY
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
        .is_err()
    {
        let _ = sender.send(Err("选区读取仍在进行，请稍后再试"));
        return;
    }
    let report = sender.clone();
    let task = std::thread::Builder::new()
        .name("qingjian-selection-uia".into())
        .spawn(move || {
            let result = std::panic::catch_unwind(|| read(process, window, &cancelled))
                .unwrap_or(Err("备用选区读取失败"));
            BUSY.store(false, Ordering::Release);
            if !cancelled.load(Ordering::Relaxed) {
                let _ = sender.send(result);
            }
        });
    if task.is_err() {
        BUSY.store(false, Ordering::Release);
        let _ = report.send(Err("无法启动选区读取线程"));
    }
}

fn foreground_matches(process: u32, window: u64, cancelled: &AtomicBool) -> bool {
    if process == 0 || window == 0 || cancelled.load(Ordering::Relaxed) {
        return false;
    }
    let current = unsafe { GetForegroundWindow() };
    let expected = HWND(window as usize as *mut std::ffi::c_void);
    let mut actual = 0;
    unsafe { GetWindowThreadProcessId(current, Some(&mut actual)) };
    current == expected && actual == process
}

fn read(process: u32, window: u64, cancelled: &AtomicBool) -> ReadResult {
    if !foreground_matches(process, window, cancelled) {
        return Err("原输入窗口已失去焦点，请重新选中后翻译");
    }
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
        .ok()
        .map_err(|_| "无法初始化备用读取")?;
    let result = (|| {
        let automation: IUIAutomation2 =
            unsafe { CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER) }
                .map_err(|_| "无法创建备用读取接口")?;
        unsafe { automation.SetConnectionTimeout(300) }.map_err(|_| "无法设置读取超时")?;
        unsafe { automation.SetTransactionTimeout(300) }.map_err(|_| "无法设置查询超时")?;
        let element =
            unsafe { automation.GetFocusedElement() }.map_err(|_| "当前应用未提供焦点元素")?;
        if unsafe { element.CurrentProcessId() }.ok() != i32::try_from(process).ok()
            || !unsafe { element.CurrentHasKeyboardFocus() }
                .map_err(|_| "无法确认输入焦点")?
                .as_bool()
            || unsafe { element.CurrentIsPassword() }
                .map_err(|_| "无法确认输入框隐私状态")?
                .as_bool()
        {
            return Err("焦点或输入框隐私校验未通过");
        }
        let pattern: IUIAutomationTextPattern =
            unsafe { element.GetCurrentPatternAs(UIA_TextPatternId) }
                .map_err(|_| "当前应用未提供可读选区（UIA）")?;
        let ranges = unsafe { pattern.GetSelection() }.map_err(|_| "当前应用无法读取选区")?;
        if unsafe { ranges.Length() }.ok() != Some(1) {
            return Err("请选择一段连续文字再翻译");
        }
        let range = unsafe { ranges.GetElement(0) }.map_err(|_| "当前选区已失效")?;
        let value = unsafe { range.GetText(501) }.map_err(|_| "读取选中文字失败")?;
        let text = String::from_utf16(&value).map_err(|_| "选中文字编码无效")?;
        if !super::valid_text(&text) {
            return Err("选区为空或超过 500 个字符单元，请缩短后重试");
        }
        if !foreground_matches(process, window, cancelled)
            || !unsafe { element.CurrentHasKeyboardFocus() }
                .map_err(|_| "无法复核输入焦点")?
                .as_bool()
            || unsafe { element.CurrentIsPassword() }
                .map_err(|_| "无法复核隐私状态")?
                .as_bool()
        {
            return Err("读取期间输入焦点已改变，请重新选择");
        }
        tracing::info!(
            units = text.encode_utf16().count(),
            "备用选区读取成功（UIA）"
        );
        Ok(text)
    })();
    unsafe { CoUninitialize() };
    result
}
