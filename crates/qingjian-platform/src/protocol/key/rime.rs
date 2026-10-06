//! Windows 语义键码转成 Rime/X11 keysym；平台不解释拼音或 Lua 快捷键。
use super::KeyEvent;

impl KeyEvent {
    pub fn rime_key(&self) -> (i32, i32) {
        let key = self.keysym.map_or_else(
            || match self.virtual_key {
                0x08 => 0xff08,
                0x09 => 0xff09,
                0x0d => 0xff0d,
                0x1b => 0xff1b,
                0x10 | 0xa0 => 0xffe1,
                0xa1 => 0xffe2,
                0x11 | 0xa2 => 0xffe3,
                0xa3 => 0xffe4,
                0x12 | 0xa4 => 0xffe9,
                0xa5 => 0xffea,
                0x14 => 0xffe5,
                0x21 => 0xff55,
                0x22 => 0xff56,
                0x23 => 0xff57,
                0x24 => 0xff50,
                0x25 => 0xff51,
                0x26 => 0xff52,
                0x27 => 0xff53,
                0x28 => 0xff54,
                0x2d => 0xff63,
                0x2e => 0xffff,
                0x5b => 0xffeb,
                0x5c => 0xffec,
                0x60..=0x69 => 0xffb0 + (self.virtual_key - 0x60) as i32,
                0x6a => 0xffaa,
                0x6b => 0xffab,
                0x6c => 0xffac,
                0x6d => 0xffad,
                0x6e => 0xffae,
                0x6f => 0xffaf,
                0x70..=0x87 => 0xffbe + (self.virtual_key - 0x70) as i32,
                _ => self.character.filter(|c| !c.is_control()).map_or_else(
                    || {
                        if (0x41..=0x5a).contains(&self.virtual_key) {
                            (self.virtual_key + if self.modifiers.shift { 0 } else { 0x20 }) as i32
                        } else {
                            self.virtual_key as i32
                        }
                    },
                    |c| {
                        if c as u32 <= 0xff {
                            c as i32
                        } else {
                            0x0100_0000 | c as i32
                        }
                    },
                ),
            },
            |sym| sym as i32,
        );
        let m = self.modifiers;
        let mask = i32::from(m.shift)
            | (i32::from(m.caps) << 1)
            | (i32::from(m.ctrl) << 2)
            | (i32::from(m.alt) << 3)
            | (i32::from(m.win) << 26)
            | (i32::from(self.release) << 30);
        (key, mask)
    }
}
