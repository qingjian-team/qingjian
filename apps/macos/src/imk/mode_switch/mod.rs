//! 单击修饰键切换的状态机；与 AppKit 解耦，便于回放左右键和组合键事件。

use qingjian_platform::SwitchKeys;

pub(super) struct ModeSwitch {
    enabled: bool,

    keys: SwitchKeys,

    english: bool,

    caps: Option<bool>,

    pending: Option<u16>,

    /// 上一次事件的公共修饰位；取消单击后仍跟踪到整体释放，防止双侧重叠误触发。
    held: usize,

    /// 逻辑状态已改变，仍需把旧组句原样上屏。
    needs_finish: bool,
}

// AppKit 的公共修饰位；IMK 事件不保证携带 IOKit 左右键的设备相关低位。
const SHIFT: usize = 1 << 17;
const CONTROL: usize = 1 << 18;
const OPTION: usize = 1 << 19;
const COMMAND: usize = 1 << 20;
const CAPS: usize = 1 << 16;
const FUNCTION: usize = 1 << 23;
const MODIFIERS: usize = SHIFT | CONTROL | OPTION | COMMAND | FUNCTION;

impl Default for ModeSwitch {
    fn default() -> Self {
        Self {
            enabled: true,
            keys: SwitchKeys::default(),
            english: false,
            caps: None,
            pending: None,
            held: 0,
            needs_finish: false,
        }
    }
}

impl ModeSwitch {
    pub fn configure(&mut self, enabled: bool, keys: SwitchKeys) {
        if self.enabled != enabled || self.keys != keys {
            self.cancel();
        }
        if self.enabled != enabled {
            self.needs_finish |= self.english();
            self.english = false;
        }
        self.enabled = enabled;
        self.keys = keys;
    }

    pub fn english(&self) -> bool {
        self.enabled && self.english
    }

    /// Caps Lock 是兼容入口：仅硬件状态变化时覆盖模式，Shift 可以覆盖亮灯时的状态。
    pub fn sync_caps(&mut self, caps: bool) -> bool {
        let before = self.english();
        if self.caps != Some(caps) {
            self.cancel();
            self.caps = Some(caps);
            self.english = self.enabled && caps;
        }
        let changed = before != self.english();
        self.needs_finish |= changed;
        changed
    }

    pub fn needs_finish(&self) -> bool {
        self.needs_finish
    }

    pub fn did_finish(&mut self) {
        self.needs_finish = false;
    }

    pub fn cancel(&mut self) {
        self.pending = None;
    }

    pub fn pending(&self) -> Option<u16> {
        self.pending
    }

    /// 焦点与鼠标变化时，从当前硬件状态重新同步，避免失焦期间漏掉松键事件。
    pub fn cancel_with_flags(&mut self, flags: usize) {
        self.cancel();
        self.held = flags & MODIFIERS;
    }

    pub fn flags(&mut self, key: u16, flags: usize) -> bool {
        let held = self.held;
        self.held = flags & MODIFIERS;
        let changed = self.sync_caps(flags & CAPS != 0);
        if !self.enabled {
            return changed;
        }
        let (group, device, enabled) = match key {
            56 => (SHIFT, 0x2, self.keys.shift),
            60 => (SHIFT, 0x4, self.keys.shift),
            59 => (CONTROL, 0x1, self.keys.control),
            62 => (CONTROL, 0x2000, self.keys.control),
            _ => {
                self.cancel();
                return changed;
            }
        };
        let down = flags & group != 0;
        if let Some(pending) = self.pending.take() {
            if pending == key && !down && flags & MODIFIERS == 0 {
                self.english = !self.english;
                self.needs_finish = true;
                return true;
            }
        } else if enabled && down && held & group == 0 && flags & MODIFIERS == group {
            let sides = if group == SHIFT { 0x6 } else { 0x2001 };
            let devices = flags & sides;
            // 有设备位时可排除双侧已按下；只有公共位时按键码及后续事件判定单击。
            if devices == 0 || devices == device {
                self.pending = Some(key);
            }
        }
        changed
    }

    /// 普通按键先取消单击判定；Ctrl+Option+Space 只在首次按下触发。
    pub fn key_down(&mut self, key: u16, flags: usize, repeat: bool) -> bool {
        self.cancel();
        self.held = flags & MODIFIERS;
        if self.enabled
            && self.keys.ctrl_alt_space
            && key == 49
            && flags & MODIFIERS == CONTROL | OPTION
        {
            if !repeat {
                self.english = !self.english;
                self.needs_finish = true;
            }
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests;
