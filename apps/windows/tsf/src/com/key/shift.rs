//! 单击 Shift 的判定，喂的是击键 sink 的 `OnTestKeyDown` / `OnTestKeyUp`（被吃掉的键也经过它们，
//! 与 `WH_KEYBOARD` 钩子不同）。按下 Shift 到抬起之间没插进别的键，就是一次单击。

use std::cell::Cell;

use windows::Win32::Foundation::LPARAM;

use super::event::is_shift;

#[derive(Default)]
pub(crate) struct ShiftTap {
    /// 按下 Shift 后还没有别的键插进来。
    alone: Cell<bool>,
}

impl ShiftTap {
    /// 任一键按下。`lparam` 第 30 位是按下前的状态（1 = 自动重复，不算新按下）。
    pub(crate) fn key_down(&self, vk: u32, lparam: LPARAM) {
        if !is_shift(vk) {
            self.alone.set(false);
        } else if (lparam.0 >> 30) & 1 == 0 {
            self.alone.set(true);
        }
    }

    /// 任一键抬起；Shift 单独抬起返回 `true`，一次抬起只算一次。
    pub(crate) fn key_up(&self, vk: u32) -> bool {
        is_shift(vk) && self.alone.replace(false)
    }
}
