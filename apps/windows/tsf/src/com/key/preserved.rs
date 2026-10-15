//! 「翻译选中文字」与「Ctrl + Alt + Space 切换中英」两个快捷键登记成 TSF **保留键**（preserved key）。
//! 带 Alt 的组合是系统键，不经击键 sink（真机：Ctrl+Alt+T 在 `OnTestKeyDown` 里从没出现过）；
//! 保留键由 TSF 在应用之前匹配、回调 `OnPreservedKey`，UWP 里也一样。翻译组合来自
//! `[shortcut] translate_selection`，激活时读一次配置（AppContainer 读不到用户目录时用缺省 Ctrl+Alt+T）；
//! 切换键来自 `[shortcut] switch_mode`，那个值由 Server 经协议下发（DLL 不读配置文件），变了就地重登记。

use windows::Win32::UI::Input::KeyboardAndMouse::{VK_OEM_PERIOD, VK_SPACE};
use windows::Win32::UI::TextServices::{
    ITfKeystrokeMgr, TF_MOD_ALT, TF_MOD_CONTROL, TF_MOD_SHIFT, TF_PRESERVEDKEY,
};
use windows::core::{GUID, Result};

use qingjian_platform::protocol::{KeyEvent, KeyModifiers};
use qingjian_platform::{Config, KeyCombo};

use crate::com::log::log;

/// 本保留键的标识，`OnPreservedKey` 按它认。
pub(crate) const GUID_TRANSLATE: GUID = GUID::from_u128(0x5c0a7b12_3d4e_4f60_8a91_2b3c4d5e6f70);

/// Ctrl + Alt + Space 中英切换键的保留键标识。
pub(crate) const GUID_SWITCH_MODE: GUID = GUID::from_u128(0x2f6b8c51_9a34_4e7d_b2c8_5d1e0f3a7b64);

/// 可配置的全 / 半角标点切换键。
pub(crate) const GUID_PUNCTUATION: GUID = GUID::from_u128(0xa8e6eafb_d2c5_4db3_96d4_5bfd6432a0d7);

/// msctf.h 的 `TF_MOD_LWIN`（windows crate 没导出）。
const TF_MOD_LWIN: u32 = 0x08;

/// 读 `%APPDATA%\Qingjian\config.toml` 里的组合；读不到 / 解析失败用缺省。
pub(crate) fn load_combo() -> KeyCombo {
    let Some(path) = qingjian_platform::dirs::config_path() else {
        return KeyCombo::TRANSLATE_DEFAULT;
    };
    match Config::load(&path) {
        Ok(config) => config.shortcut.translate_selection,
        Err(error) => {
            log(&format!("读配置取翻译快捷键失败，用缺省: {error}"));
            KeyCombo::TRANSLATE_DEFAULT
        }
    }
}

/// Ctrl + Alt + Space 的 `TF_PRESERVEDKEY`。不用 Ctrl + Space：中文 Windows 把它绑成系统的
/// 「输入法/非输入法切换」，系统先截走，保留键收不到。
fn switch_mode_key() -> TF_PRESERVEDKEY {
    TF_PRESERVEDKEY {
        uVKey: VK_SPACE.0 as u32,
        uModifiers: TF_MOD_CONTROL | TF_MOD_ALT,
    }
}

/// 登记 Ctrl + Alt + Space 为中英切换保留键（`switch_mode` 勾了它时）。
pub(crate) fn register_switch_mode(keystroke: &ITfKeystrokeMgr, tid: u32) -> Result<()> {
    let description: Vec<u16> = "切换中英文（青简）".encode_utf16().collect();
    unsafe { keystroke.PreserveKey(tid, &GUID_SWITCH_MODE, &switch_mode_key(), &description) }
}

pub(crate) fn unregister_switch_mode(keystroke: &ITfKeystrokeMgr) {
    let _ = unsafe { keystroke.UnpreserveKey(&GUID_SWITCH_MODE, &switch_mode_key()) };
}

fn preserved_key(combo: KeyCombo) -> TF_PRESERVEDKEY {
    let m = combo.modifiers;
    let mut modifiers = 0;
    if m.control {
        modifiers |= TF_MOD_CONTROL;
    }
    if m.option {
        modifiers |= TF_MOD_ALT;
    }
    if m.shift {
        modifiers |= TF_MOD_SHIFT;
    }
    if m.command {
        modifiers |= TF_MOD_LWIN;
    }
    TF_PRESERVEDKEY {
        uVKey: virtual_key(combo),
        uModifiers: modifiers,
    }
}

pub(crate) fn register(keystroke: &ITfKeystrokeMgr, tid: u32, combo: KeyCombo) -> Result<()> {
    let key = preserved_key(combo);
    let description: Vec<u16> = "翻译选中文字".encode_utf16().collect();
    unsafe { keystroke.PreserveKey(tid, &GUID_TRANSLATE, &key, &description) }
}

pub(crate) fn unregister(keystroke: &ITfKeystrokeMgr, combo: KeyCombo) {
    let key = preserved_key(combo);
    let _ = unsafe { keystroke.UnpreserveKey(&GUID_TRANSLATE, &key) };
}

/// 保留键命中时喂给 Server 的按键：Router 按字符 + 物理修饰键与配置比对。
pub(crate) fn key_event(combo: KeyCombo, english_mode: bool) -> KeyEvent {
    let m = combo.modifiers;
    KeyEvent::new(
        virtual_key(combo),
        Some(combo.key),
        KeyModifiers {
            ctrl: m.control,
            shift: m.shift,
            alt: m.option,
            win: m.command,
            caps: false,
            english_mode,
        },
    )
}

pub(crate) fn register_punctuation(
    keystroke: &ITfKeystrokeMgr,
    tid: u32,
    combo: KeyCombo,
) -> Result<()> {
    let description: Vec<u16> = "切换全半角标点（青简）".encode_utf16().collect();
    unsafe { keystroke.PreserveKey(tid, &GUID_PUNCTUATION, &preserved_key(combo), &description) }
}

pub(crate) fn unregister_punctuation(keystroke: &ITfKeystrokeMgr, combo: KeyCombo) {
    let _ = unsafe { keystroke.UnpreserveKey(&GUID_PUNCTUATION, &preserved_key(combo)) };
}

fn virtual_key(combo: KeyCombo) -> u32 {
    if combo.key == '.' {
        u32::from(VK_OEM_PERIOD.0)
    } else {
        combo.key.to_ascii_uppercase() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::preserved_key;
    use windows::Win32::UI::Input::KeyboardAndMouse::VK_OEM_PERIOD;
    use windows::Win32::UI::TextServices::{TF_MOD_CONTROL, TF_MOD_SHIFT};

    #[test]
    fn punctuation_combo_uses_period_virtual_key_instead_of_delete() {
        // '.' 的 ASCII 码是 0x2E（VK_DELETE），不能拿来登记句号保留键。
        let key = preserved_key("ctrl+shift+.".parse().unwrap());
        assert_eq!(key.uVKey, u32::from(VK_OEM_PERIOD.0));
        assert_eq!(key.uModifiers, TF_MOD_CONTROL | TF_MOD_SHIFT);
        assert_eq!(preserved_key("ctrl+t".parse().unwrap()).uVKey, b'T' as u32);
    }
}
