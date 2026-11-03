//! Windows 简体中文输入法的 Ctrl + Space（输入法 / 非输入法切换）：状态由系统保存，不写进青简配置，也会影响微软拼音。
//! 另有全局快捷键的占用检查。

use qingjian_platform::KeyCombo;
use windows::Win32::UI::Input::Ime::{
    IME_CHOTKEY_IME_NONIME_TOGGLE, ImmGetHotKey, ImmSetHotKey, MOD_IGNORE_ALL_MODIFIER, MOD_LEFT,
    MOD_RIGHT,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HKL, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN, RegisterHotKey,
    UnregisterHotKey, VK_SPACE,
};

use crate::i18n::tr;

/// 这个组合现在能注册成全局快捷键：试着注册一下马上放掉。系统保留的（Win + V）与别的程序占着的都注册不上。
pub fn combo_available(combo: KeyCombo) -> bool {
    let mut modifiers: HOT_KEY_MODIFIERS = MOD_NOREPEAT;
    for (held, modifier) in [
        (combo.modifiers.control, MOD_CONTROL),
        (combo.modifiers.shift, MOD_SHIFT),
        (combo.modifiers.option, MOD_ALT),
        (combo.modifiers.command, MOD_WIN),
    ] {
        if held {
            modifiers |= modifier;
        }
    }
    let key = if combo.key == '.' {
        0xBE
    } else {
        u32::from(combo.key.to_ascii_uppercase())
    };
    const PROBE_ID: i32 = 0x5157;
    let registered = unsafe { RegisterHotKey(None, PROBE_ID, modifiers, key) }.is_ok();
    if registered {
        let _ = unsafe { UnregisterHotKey(None, PROBE_ID) };
    }
    registered
}

pub fn ctrl_space_enabled() -> bool {
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

pub fn set_ctrl_space(enabled: bool) -> Result<(), String> {
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
        Err(tr("language.ctrl_space_failed").to_owned())
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
