//! macOS keyCode 与设备修饰键位转为原生事件；不依赖 AppKit，便于跨平台验证。
use super::{KeyEvent, KeyModifiers};

impl KeyEvent {
    /// keyCode 来自 HIToolbox，flags 保留 IOLLEvent.h 的左右键设备位。
    pub fn from_macos_rime(
        code: u16,
        character: Option<char>,
        flags: u64,
        key_up: bool,
        flags_changed: bool,
    ) -> Option<Self> {
        let symbol = match code {
            36 => 0xff0d,
            76 => 0xff8d,
            48 => 0xff09,
            51 => 0xff08,
            117 => 0xffff,
            53 => 0xff1b,
            114 => 0xff63,
            123 => 0xff51,
            124 => 0xff53,
            125 => 0xff54,
            126 => 0xff52,
            115 => 0xff50,
            119 => 0xff57,
            116 => 0xff55,
            121 => 0xff56,
            122 => 0xffbe,
            120 => 0xffbf,
            99 => 0xffc0,
            118 => 0xffc1,
            96 => 0xffc2,
            97 => 0xffc3,
            98 => 0xffc4,
            100 => 0xffc5,
            101 => 0xffc6,
            109 => 0xffc7,
            103 => 0xffc8,
            111 => 0xffc9,
            105 => 0xffca,
            107 => 0xffcb,
            113 => 0xffcc,
            106 => 0xffcd,
            64 => 0xffce,
            79 => 0xffcf,
            80 => 0xffd0,
            90 => 0xffd1,
            56 => 0xffe1,
            60 => 0xffe2,
            59 => 0xffe3,
            62 => 0xffe4,
            58 => 0xffe9,
            61 => 0xffea,
            55 => 0xffeb,
            54 => 0xffec,
            57 => 0xffe5,
            82 => 0xffb0,
            83..=89 => 0xffb1 + u32::from(code - 83),
            91..=92 => 0xffb8 + u32::from(code - 91),
            65 => 0xffae,
            67 => 0xffaa,
            69 => 0xffab,
            71 => 0xff0b,
            75 => 0xffaf,
            78 => 0xffad,
            81 => 0xffbd,
            _ => {
                let character = character? as u32;
                if character <= 0xff {
                    character
                } else {
                    0x0100_0000 | character
                }
            }
        };
        let modifiers = KeyModifiers {
            shift: flags & (1 << 17) != 0,
            caps: flags & (1 << 16) != 0,
            ctrl: flags & (1 << 18) != 0,
            alt: flags & (1 << 19) != 0,
            win: flags & (1 << 20) != 0,
            ..KeyModifiers::default()
        };
        let mut event = Self::new(u32::from(code), character, modifiers);
        event.keysym = Some(symbol);
        event.release = key_up || (flags_changed && modifier_released(code, flags));
        Some(event)
    }
}

fn modifier_released(code: u16, flags: u64) -> bool {
    // 聚合 Shift 位在另一侧仍按住时不会清除，释放判定必须读本键的设备位。
    // 位定义：https://github.com/apple-oss-distributions/IOHIDFamily/blob/main/IOHIDSystem/IOKit/hidsystem/IOLLEvent.h
    let mask = match code {
        56 => 0x02,
        60 => 0x04,
        59 => 0x01,
        62 => 0x2000,
        58 => 0x20,
        61 => 0x40,
        55 => 0x08,
        54 => 0x10,
        57 => 1 << 16,
        _ => return false,
    };
    flags & mask == 0
}
