//! 按键相关：TSF 虚拟键码到协议 [`KeyEvent`](qingjian_platform::protocol::KeyEvent) 的翻译（[`event`]）、
//! 单击中英切换键的判定（[`tap`]，键来自 `[shortcut] switch_mode`）、「翻译选中文字」快捷键的保留键登记（[`preserved`]）、
//! 给不认 TSF 选区的输入框模拟方向键挪光标（[`inject`]）。

pub(crate) mod event;
pub(crate) mod inject;
mod layout;
pub(crate) mod preserved;
mod tap;

pub(crate) use self::tap::KeyTap;
