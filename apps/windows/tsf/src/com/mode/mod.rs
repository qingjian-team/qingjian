//! 中 / 英输入模式：这里写 TSF 的「转换模式」compartment，系统任务栏据此显示「中」或「英」，
//! 只翻 `TF_CONVERSIONMODE_NATIVE` 位，其它位（全 / 半角等）保留；另读「输入法开 / 关」compartment，
//! 系统的「输入法/非输入法切换」热键翻的是它。反向同步在 [`sink`]；模式本身与语言栏按钮在 [`state`] / [`button`]。

mod button;
mod icon;
pub(crate) mod menu;
pub(crate) mod sink;
mod state;

use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::TextServices::{
    GUID_COMPARTMENT_KEYBOARD_INPUTMODE_CONVERSION, GUID_COMPARTMENT_KEYBOARD_OPENCLOSE,
    ITfCompartment, ITfCompartmentMgr, ITfThreadMgr, TF_CONVERSIONMODE_NATIVE,
};
use windows::core::{Interface, Result};

use crate::com::log::log;

pub(crate) use self::button::ModeButton;
pub(crate) use self::icon::taskbar_is_light;
pub(crate) use self::state::ModeState;

const NATIVE: i32 = TF_CONVERSIONMODE_NATIVE as i32;

/// 失败只记日志：指示器不动不影响打字。
pub(crate) fn set_indicator(thread_mgr: &ITfThreadMgr, tid: u32, english: bool) {
    if let Err(error) = write_conversion_mode(thread_mgr, tid, english) {
        log(&format!("设置中英指示器失败: {error}"));
    }
}

pub(crate) fn conversion_compartment(thread_mgr: &ITfThreadMgr) -> Result<ITfCompartment> {
    let mgr: ITfCompartmentMgr = thread_mgr.cast()?;
    unsafe { mgr.GetCompartment(&GUID_COMPARTMENT_KEYBOARD_INPUTMODE_CONVERSION) }
}

/// `NATIVE` 未点亮 = 英文。读不到按中文起算。
pub(crate) fn is_english(compartment: &ITfCompartment) -> bool {
    read_mode(compartment) & NATIVE == 0
}

/// 「输入法开 / 关」compartment：系统的「输入法/非输入法切换」热键（缺省 Ctrl+Space）翻的是它。
/// 只是一个状态位，关着时按键照样送来；新线程里缺省是关。
pub(crate) fn openclose_compartment(thread_mgr: &ITfThreadMgr) -> Result<ITfCompartment> {
    let mgr: ITfCompartmentMgr = thread_mgr.cast()?;
    unsafe { mgr.GetCompartment(&GUID_COMPARTMENT_KEYBOARD_OPENCLOSE) }
}

/// 值为 0 = 关着。没设过 / 读不到按开着算。
pub(crate) fn is_keyboard_open(compartment: &ITfCompartment) -> bool {
    read_i32(compartment).is_none_or(|value| value != 0)
}

/// 跟着模式写（中文开、英文关），系统 Ctrl + Space 下一次按下才是真的切换。已是目标值就不写，失败只记日志。
pub(crate) fn set_keyboard_open(thread_mgr: &ITfThreadMgr, tid: u32, open: bool) {
    let Ok(compartment) = openclose_compartment(thread_mgr) else {
        return;
    };
    if is_keyboard_open(&compartment) == open {
        return;
    }
    if let Err(error) = unsafe { compartment.SetValue(tid, &VARIANT::from(i32::from(open))) } {
        log(&format!("写输入法开关失败: {error}"));
    }
}

fn write_conversion_mode(thread_mgr: &ITfThreadMgr, tid: u32, english: bool) -> Result<()> {
    let compartment = conversion_compartment(thread_mgr)?;
    let current = read_mode(&compartment);
    let next = if english {
        current & !NATIVE
    } else {
        current | NATIVE
    };
    unsafe { compartment.SetValue(tid, &VARIANT::from(next)) }
}

/// 没设过 / 类型不对时按中文（`NATIVE` 亮）。
fn read_mode(compartment: &ITfCompartment) -> i32 {
    read_i32(compartment).unwrap_or(NATIVE)
}

fn read_i32(compartment: &ITfCompartment) -> Option<i32> {
    unsafe { compartment.GetValue() }
        .and_then(|variant| i32::try_from(&variant))
        .ok()
}
