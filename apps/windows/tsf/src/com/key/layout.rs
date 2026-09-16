//! 标点 / 数字按当前键盘布局解析，测键时不改变 dead key 状态。

use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyboardLayout, GetKeyboardState, HKL, MAPVK_VK_TO_VSC, MapVirtualKeyExW, ToUnicodeEx,
};

pub(super) fn character(vk: u32) -> Option<char> {
    // 保持原有键码范围：主键盘数字、OEM 标点和空格。
    if !matches!(vk, 0x30..=0x39 | 0xBA..=0xC0 | 0xDB..=0xDE | 0x20) {
        return None;
    }
    let mut state = [0; 256];
    unsafe { GetKeyboardState(&mut state) }.ok()?;
    resolve(vk, &state, unsafe { GetKeyboardLayout(0) })
}

fn resolve(vk: u32, state: &[u8; 256], layout: HKL) -> Option<char> {
    let scan = unsafe { MapVirtualKeyExW(vk, MAPVK_VK_TO_VSC, Some(layout)) };
    let mut buffer = [0; 8];
    // Windows 10 1607 起，bit 2 可避免测键修改 dead key 状态。
    let count = unsafe { ToUnicodeEx(vk, scan, state, &mut buffer, 1 << 2, Some(layout)) };
    // 仅接受单个 UTF-16 单元；dead key、无映射和较长结果不生成字符。
    if count != 1 {
        return None;
    }
    char::from_u32(u32::from(buffer[0]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        KLF_NOTELLSHELL, LoadKeyboardLayoutW, VK_SHIFT,
    };
    use windows::core::w;

    #[test]
    fn punctuation_and_digits_follow_layout() {
        // 不加 KLF_ACTIVATE，不切换用户正在使用的布局。
        let us = unsafe { LoadKeyboardLayoutW(w!("00000409"), KLF_NOTELLSHELL) }.unwrap();
        let de = unsafe { LoadKeyboardLayoutW(w!("00000407"), KLF_NOTELLSHELL) }.unwrap();
        let mut state = [0; 256];
        assert_eq!(resolve(0xBA, &state, us), Some(';'));
        assert_eq!(resolve(0x32, &state, de), Some('2'));
        assert_eq!(resolve(0x20, &state, us), Some(' '));
        state[VK_SHIFT.0 as usize] = 0x80;
        assert_eq!(resolve(0xBA, &state, us), Some(':'));
        assert_eq!(resolve(0xBB, &state, us), Some('+'));
        assert_eq!(resolve(0xBB, &state, de), Some('*'));
        assert_eq!(resolve(0x32, &state, us), Some('@'));
        assert_eq!(resolve(0x32, &state, de), Some('"'));
    }

    #[test]
    fn dead_key_lookup_does_not_change_next_character() {
        let intl = unsafe { LoadKeyboardLayoutW(w!("00020409"), KLF_NOTELLSHELL) }.unwrap();
        let state = [0; 256];
        for _ in 0..2 {
            assert_eq!(resolve(0xDE, &state, intl), None);
            // 每次预读后立即检查，避免两次 dead key 抵消副作用。
            assert_eq!(resolve(0x45, &state, intl), Some('e'));
        }
        assert_eq!(resolve(0, &state, intl), None);
        let mut buffer = [0; 8];
        // 与上面的重音检查串行：内核键盘缓冲会让并行测试互相影响。
        let dead = unsafe { ToUnicodeEx(0xDE, 0, &state, &mut buffer, 0, Some(intl)) };
        let result = resolve(0x31, &state, intl);
        // 清掉待组合的重音后再断言。
        let count = unsafe { ToUnicodeEx(0x31, 0, &state, &mut buffer, 0, Some(intl)) };
        assert!(dead < 0);
        assert_eq!(count, 2);
        assert_eq!(result, None);
    }

    #[test]
    fn other_keys_are_not_resolved() {
        for vk in [0x41, 0x60, 0x0D, 0x70] {
            assert_eq!(character(vk), None);
        }
    }

    #[test]
    fn event_reads_thread_layout_and_keyboard_state() {
        use crate::com::key::event::to_key_event;
        use crate::com::service::TextService;
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            ActivateKeyboardLayout, SetKeyboardState, VK_CAPITAL, VK_CONTROL, VK_LSHIFT,
        };
        use windows::Win32::UI::TextServices::ITfKeyEventSink;
        use windows::core::ComObject;

        let original_layout = unsafe { GetKeyboardLayout(0) };
        let mut original_state = [0; 256];
        unsafe { GetKeyboardState(&mut original_state) }.unwrap();
        // 仅修改测试线程；即使断言失败，也先恢复布局和按键状态。
        let result = std::panic::catch_unwind(|| {
            let service = ComObject::new(TextService::new());
            let sink: ITfKeyEventSink = service.to_interface();
            for (name, expected) in [(w!("00000409"), '+'), (w!("00000407"), '*')] {
                let layout = unsafe { LoadKeyboardLayoutW(name, KLF_NOTELLSHELL) }.unwrap();
                unsafe { ActivateKeyboardLayout(layout, Default::default()) }.unwrap();
                let mut state = [0; 256];
                state[VK_SHIFT.0 as usize] = 0x80;
                state[VK_LSHIFT.0 as usize] = 0x80;
                unsafe { SetKeyboardState(&state) }.unwrap();
                for _ in 0..2 {
                    let event = to_key_event(0xBB, true);
                    assert_eq!(event.character, Some(expected));
                    assert!(event.modifiers.shift);
                    assert!(event.modifiers.english_mode);
                    assert!(
                        unsafe { sink.OnTestKeyDown(None, WPARAM(0xBB), LPARAM(0)) }
                            .unwrap()
                            .as_bool()
                    );
                }
                state[VK_SHIFT.0 as usize] = 0;
                state[VK_LSHIFT.0 as usize] = 0;
                state[VK_CAPITAL.0 as usize] = 1;
                state[VK_CONTROL.0 as usize] = 0x80;
                unsafe { SetKeyboardState(&state) }.unwrap();
                let event = to_key_event(0x31, false);
                assert!(!event.modifiers.shift);
                assert!(event.modifiers.caps);
                assert!(event.modifiers.ctrl);
                assert_eq!(event.virtual_key, 0x31);
                assert_eq!(to_key_event(0x41, false).character, Some('A'));
                assert!(
                    !unsafe { sink.OnTestKeyDown(None, WPARAM(0x31), LPARAM(0)) }
                        .unwrap()
                        .as_bool()
                );
            }
        });
        unsafe { ActivateKeyboardLayout(original_layout, Default::default()) }.unwrap();
        unsafe { SetKeyboardState(&original_state) }.unwrap();
        result.unwrap();
    }
}
