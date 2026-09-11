//! 按键相关：TSF 虚拟键码到协议 [`KeyEvent`](qingjian_platform::protocol::KeyEvent) 的翻译（[`event`]）、
//! 单击 Shift 切中英的判定（[`shift`]）、「翻译选中文字」快捷键的保留键登记（[`preserved`]）。

pub(crate) mod event;
pub(crate) mod preserved;
mod shift;

pub(crate) use self::shift::ShiftTap;
