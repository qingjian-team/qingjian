//! 保留 TSF 扫描码与 extended 位，以区分左右修饰键和小键盘。
use super::event::to_key_event;
use qingjian_platform::protocol::KeyEvent;
use windows::Win32::Foundation::LPARAM;

pub(crate) fn to_rime_key_event(vk: u32, lparam: LPARAM, english: bool) -> KeyEvent {
    let mut event = to_key_event(vk, english);
    event.keysym = physical_keysym(vk, lparam.0 as usize);
    event
}

// WM_KEYDOWN / KEYUP 的 16..23 位为扫描码，24 位标记扩展键；释放不改变键身份。
fn physical_keysym(vk: u32, bits: usize) -> Option<u32> {
    let scan = (bits >> 16) & 0xff;
    let extended = bits & (1 << 24) != 0;
    Some(match vk {
        0x10 => {
            if scan == 0x36 {
                0xffe2
            } else {
                0xffe1
            }
        }
        0x11 => {
            if extended {
                0xffe4
            } else {
                0xffe3
            }
        }
        0x12 => {
            if extended {
                0xffea
            } else {
                0xffe9
            }
        }
        0x0d if extended => 0xff8d,
        0x0c => 0xff9d,
        0x21 if !extended => 0xff9a,
        0x22 if !extended => 0xff9b,
        0x23 if !extended => 0xff9c,
        0x24 if !extended => 0xff95,
        0x25 if !extended => 0xff96,
        0x26 if !extended => 0xff97,
        0x27 if !extended => 0xff98,
        0x28 if !extended => 0xff99,
        0x2d if !extended => 0xff9e,
        0x2e if !extended => 0xff9f,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tsf_scancodes_preserve_each_modifier_on_press_and_release() {
        for (vk, scan, extended, expected) in [
            (0x10, 0x2a, false, 0xffe1),
            (0x10, 0x36, false, 0xffe2),
            (0x11, 0x1d, false, 0xffe3),
            (0x11, 0x1d, true, 0xffe4),
            (0x12, 0x38, false, 0xffe9),
            (0x12, 0x38, true, 0xffea),
        ] {
            for release in [false, true] {
                let bits =
                    (scan << 16) | (usize::from(extended) << 24) | (usize::from(release) << 31);
                let mut event =
                    to_rime_key_event(vk, windows::Win32::Foundation::LPARAM(bits as isize), false);
                event.release = release;
                let (key, mask) = event.rime_key();
                assert_eq!(key, expected);
                assert_eq!(mask & (1 << 30) != 0, release);
            }
        }
    }

    #[test]
    fn keypad_enter_and_navigation_keep_their_own_identity() {
        let main = to_rime_key_event(0x0d, LPARAM(0x1c << 16), false);
        let pad = to_rime_key_event(0x0d, LPARAM((0x1c << 16) | (1 << 24)), false);
        assert_eq!(main.rime_key().0, 0xff0d);
        assert_eq!(pad.rime_key().0, 0xff8d);
        for (vk, standard, keypad) in [
            (0x24, 0xff50, 0xff95),
            (0x2d, 0xff63, 0xff9e),
            (0x2e, 0xffff, 0xff9f),
        ] {
            assert_eq!(
                to_rime_key_event(vk, LPARAM(1 << 24), false).rime_key().0,
                standard
            );
            assert_eq!(to_rime_key_event(vk, LPARAM(0), false).rime_key().0, keypad);
        }
    }
}
