//! 中 / 英模式：Caps Lock 与切换键（`[shortcut] switch_mode`）共同维护的进程级状态。
//!
//! 模式是**状态**而不是每次按键从 Caps Lock 现读：切换键切到英文后 Caps Lock 还灭着，
//! 下次按键不能又读回中文。所以记一份 `mode_english`，Caps Lock 变化时同步过来，切换键单独翻转它。

use qingjian_platform::{Modifiers, SwitchKey};

use super::Host;
use crate::imk::modifiers;

impl Host {
    /// 处理一个切换键相关事件（Shift / Ctrl 的按下抬起，或 Ctrl + Alt + Space 按下）。
    ///
    /// 先按 Caps Lock 同步模式（用户刚按了 Caps Lock 就以它为准），再喂单击 / 组合键状态机；
    /// 触发切换就翻转模式并刷状态项。返回是否吞掉这个事件——组合键吞掉（不当成空格），
    /// 单击的修饰键事件照常交还给应用。
    pub fn handle_switch_key(&mut self, key_code: u16, pressed: Modifiers, is_up: bool) -> bool {
        self.sync_mode_from_caps();
        let fired = if is_up {
            self.switch_tap.key_up(key_code, self.switch_keys)
        } else {
            self.switch_tap
                .key_down(key_code, pressed, self.switch_keys)
        };
        // 内置英文模式关着（固定中文）：切换键不生效，事件照常交还应用
        if !fired || !self.switch_enabled {
            return false;
        }
        // 组合键按下即触发要吞掉；单击是修饰键的抬起，不吞
        let swallow = !is_up;
        self.toggle_switch_mode();
        swallow
    }

    /// 处理一个修饰键状态变化（来自 CGEventTap 的 flagsChanged）。
    ///
    /// IMK 的 `handleEvent` 不收修饰键（Shift / Ctrl）事件，单击切换键只能从这里判：先按 Caps Lock
    /// 同步模式（与 [`Self::handle_switch_key`] 一致），再按差量找出变了的切换键、是按下还是抬起，
    /// 喂给状态机；触发单击就翻转模式。Option / Command 不是切换键，忽略。
    pub fn handle_modifier_change(&mut self, current: Modifiers) {
        self.sync_mode_from_caps();
        let prev = self.last_switch_modifiers;
        self.last_switch_modifiers = current;
        if prev.shift != current.shift {
            self.apply_modifier_tap(SwitchKey::Shift, current.shift);
        }
        if prev.control != current.control {
            self.apply_modifier_tap(SwitchKey::Control, current.control);
        }
    }

    /// 一个切换键按下 / 抬起：喂状态机，触发单击就翻转模式；内置英文模式关着时不切。
    fn apply_modifier_tap(&mut self, key: SwitchKey, pressed: bool) {
        let fired = self
            .switch_tap
            .modifier_change(key, pressed, self.switch_keys);
        if fired && self.switch_enabled {
            self.toggle_switch_mode();
        }
    }

    /// 按 Caps Lock 同步模式：它变了就是用户刚按了 Caps Lock，模式跟着它。
    ///
    /// 每个按键处理前调一次；状态项的定时器也轮询它——Caps Lock 的变化不会作为按键送来，
    /// 用户按了 Caps Lock 还没打字时靠它把状态项刷对。
    pub fn sync_mode_from_caps(&mut self) {
        let caps = modifiers::caps_lock_on();
        if caps == self.last_caps {
            return;
        }
        self.last_caps = caps;
        // 内置英文模式关着时 Caps Lock 也进不了英文
        let english = caps && self.switch_enabled;
        if english != self.mode_english {
            self.mode_english = english;
            self.indicator.update(self.mode_english);
        }
    }

    /// 翻转模式；内置英文模式关着时翻不到英文。
    fn toggle_switch_mode(&mut self) {
        if !self.switch_enabled {
            return;
        }
        self.mode_english = !self.mode_english;
        tracing::info!(english = self.mode_english, "中英模式已切换");
        self.indicator.update(self.mode_english);
    }

    /// 当前中 / 英模式（`true` 是英文）。组句分流、标点、候选都按它走。
    pub fn mode_english(&self) -> bool {
        self.mode_english
    }
}
