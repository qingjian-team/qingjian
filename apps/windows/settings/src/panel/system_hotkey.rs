//! 读取和设置 Windows 的简体中文输入法 Ctrl + Space，状态由系统保存，不写入青简配置。

use windows::Win32::UI::Input::Ime::{
    IME_CHOTKEY_IME_NONIME_TOGGLE, ImmGetHotKey, ImmSetHotKey, MOD_IGNORE_ALL_MODIFIER, MOD_LEFT,
    MOD_RIGHT,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{HKL, MOD_CONTROL, VK_SPACE};

pub(super) fn ctrl_space_enabled() -> bool {
    let mut modifiers = 0;
    let mut key = 0;
    let mut layout = HKL::default();
    unsafe {
        ImmGetHotKey(
            IME_CHOTKEY_IME_NONIME_TOGGLE.0,
            &mut modifiers,
            &mut key,
            &mut layout,
        )
    }
    .as_bool()
        && is_ctrl_space(modifiers, key, layout)
}

fn is_ctrl_space(modifiers: u32, key: u32, layout: HKL) -> bool {
    // 左右 Ctrl 标记不改变组合；忽略修饰键或叠加 Alt / Shift / Win 的绑定不能显示为 Ctrl + Space。
    key == u32::from(VK_SPACE.0)
        && modifiers & (0x0F | MOD_IGNORE_ALL_MODIFIER) == MOD_CONTROL.0
        && layout == HKL::default()
}

pub(super) fn set_ctrl_space(enabled: bool) -> Result<(), String> {
    // 防止用户取消勾选前，别的程序已把此槽位改成另一种组合。
    if !enabled && !ctrl_space_enabled() {
        return Ok(());
    }
    let (modifiers, key) = if enabled {
        (MOD_CONTROL.0 | MOD_LEFT | MOD_RIGHT, u32::from(VK_SPACE.0))
    } else {
        (0, 0)
    };
    let changed = unsafe {
        ImmSetHotKey(
            IME_CHOTKEY_IME_NONIME_TOGGLE.0,
            modifiers,
            key,
            HKL::default(),
        )
    }
    .as_bool();
    if changed && ctrl_space_enabled() == enabled {
        Ok(())
    } else {
        Err("Windows 未能更新 Ctrl + Space，请重新打开设置后重试。".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_system_ctrl_space_binding_is_checked() {
        for side in [0, MOD_LEFT, MOD_RIGHT, MOD_LEFT | MOD_RIGHT] {
            assert!(is_ctrl_space(MOD_CONTROL.0 | side, 0x20, HKL::default()));
        }
        for modifiers in [0, 0x01, 0x03, 0x06, 0x0A, MOD_IGNORE_ALL_MODIFIER | 0x02] {
            assert!(!is_ctrl_space(modifiers, 0x20, HKL::default()));
        }
        assert!(!is_ctrl_space(0x02, 0, HKL::default()));
        assert!(!is_ctrl_space(0x02, 0x20, HKL(std::ptr::dangling_mut())));
    }
}
