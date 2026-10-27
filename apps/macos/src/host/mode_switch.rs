//! 单击 Shift 切换中 / 英（配置 `[shortcut] switch_mode` 勾了 shift 时）。
//!
//! 边沿从修饰键事件来：控制器在 `recognizedEvents:` 里声明了 flagsChanged
//! （缺省只回 `NSKeyDownMask`，不声明就一个修饰键事件都收不到，Shift 的按下 / 抬起只能靠轮询采样，
//! 而采样在「按下与抬起落在同一拍之间」时会把整次点击漏掉），拿到精确边沿后在这里判定
//! 「按下到抬起之间没有别的键插进来」= 一次单击。规则与 Windows 的 `com/key/tap.rs` 相同，但那边能看到
//! 被系统热键吃掉的键并作废这次单击，macOS 看不到（系统热键不进 `handleEvent:`），
//! 所以「按住 Shift 按了系统快捷键再松开」会被误判成一次单击。
//! 勾上之后中英由本模块决定，Caps Lock 退化成只管大小写。

use std::time::{Duration, Instant};

use crate::imk::modifiers;

use super::Host;

/// 切换之后这么久内的 Shift 边沿都不算：有些外接键盘松开 Shift 后 8–10 ms 又补一对按下 / 抬起，
/// 不挡的话一次单击会切过去又切回来。人手连按两次在 200 ms 以上。
const TOGGLE_COOLDOWN: Duration = Duration::from_millis(150);

/// 按住超过这么久再松开不算单击：鼠标事件不进 IMK 会话，按住 Shift 拖选文字时没有别的键能作废这次单击。
const TAP_WINDOW: Duration = Duration::from_millis(300);

/// 左右两个 Shift 的键码。只有 Shift 自己不算「插进来的别的键」：按下它本来就是这次单击的一部分；
/// 别的修饰键（Cmd / Ctrl / Option / Caps Lock）按下一律作废，与 Windows 的 `tap_key` 同口径。
fn is_shift_key(key_code: u16) -> bool {
    matches!(key_code, 56 | 60)
}

/// 单击 Shift 的判定与它切出来的持久英文模式。
#[derive(Default)]
pub struct ModeSwitch {
    /// 配置里勾了「单击 Shift」（`[shortcut] switch_mode` 含 shift）。
    enabled: bool,

    /// Shift 按下之后还没有别的键插进来时，按下的时刻；抬起时没超过 [`TAP_WINDOW`] 就算一次单击。
    pending: Option<Instant>,

    /// 上次切换的时刻，[`TOGGLE_COOLDOWN`] 内的 Shift 边沿当抖动丢掉。
    toggled_at: Option<Instant>,

    /// 单击 Shift 切出来的持久英文模式；`enabled` 为假时无意义（`set_enabled(false)` 会清零）。
    english: bool,
}

impl ModeSwitch {
    /// 配置里勾了「单击 Shift」。
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// 单击 Shift 切出来的英文模式。
    pub fn english(&self) -> bool {
        self.english
    }

    /// 配置里的开关变了。
    pub fn set_enabled(&mut self, enabled: bool) {
        if self.enabled == enabled {
            return;
        }
        self.enabled = enabled;
        // 关掉时退回中文，免得留着上次切出来的英文模式
        if !enabled {
            self.english = false;
        }
    }

    /// 焦点切换 / 停用：忘掉这次还没抬起的 Shift，
    /// 免得「按着 Shift 切走再切回、然后松开」被当成一次单击。
    pub fn reset(&mut self) {
        self.pending = None;
    }

    /// 任一普通按键按下：Shift 按下到抬起之间插了别的键，这次不算单击。
    pub fn note_key(&mut self) {
        self.pending = None;
    }

    /// 修饰键事件：`key_code` 那次按下（`down`）。构成一次「单击 Shift」返回 `true`。
    pub fn shift_event(&mut self, key_code: u16, down: bool) -> bool {
        self.shift_event_at(key_code, down, Instant::now())
    }

    fn shift_event_at(&mut self, key_code: u16, down: bool, now: Instant) -> bool {
        if !self.enabled {
            return false;
        }
        // 别的修饰键（Cmd / Ctrl / Option / Caps Lock）：这次不算单击
        if !is_shift_key(key_code) {
            self.pending = None;
            return false;
        }
        if self
            .toggled_at
            .is_some_and(|at| now.duration_since(at) < TOGGLE_COOLDOWN)
        {
            self.pending = None;
            return false;
        }
        if down {
            self.pending = Some(now);
            return false;
        }
        let clicked = self
            .pending
            .take()
            .is_some_and(|at| now.duration_since(at) <= TAP_WINDOW);
        if clicked {
            self.english = !self.english;
            self.toggled_at = Some(now);
        }
        clicked
    }
}

impl Host {
    /// 中英模式：勾了「单击 Shift」时由 Shift 的切换决定（Caps Lock 退化成只管大小写），没勾看 Caps Lock。
    pub fn english_mode(&self) -> bool {
        if self.mode_switch.enabled() {
            self.mode_switch.english()
        } else {
            modifiers::caps_lock_on()
        }
    }

    /// 英文模式下一个字母要不要大写：勾了「单击 Shift」时 Caps Lock 只管大小写（与 Shift 异或）；
    /// 没勾时 Caps Lock 就是中英开关，不参与大小写。
    pub fn english_letter_uppercase(&self) -> bool {
        if self.mode_switch.enabled() {
            modifiers::shift_down() != modifiers::caps_lock_on()
        } else {
            modifiers::shift_down()
        }
    }

    /// 收到了修饰键事件（`recognizedEvents:` 里的 flagsChanged）：Shift 的按下 / 抬起在这里判定，
    /// 这次切换了中英返回 `true`。
    pub fn shift_event(&mut self, key_code: u16, down: bool) -> bool {
        if !self.mode_switch.shift_event(key_code, down) {
            return false;
        }
        tracing::info!(english = self.english_mode(), "单击 Shift 切换中英");
        self.refresh_mode_ui();
        self.render();
        true
    }

    /// 中英模式或 Caps Lock 可能变了：刷新状态条与候选窗口的输入状态，变了就弹一次右上角提示。
    ///
    /// 中英变了显示「中」/「英」（优先）；只有大小写变了才显示「大写」/「小写」——后者出现在勾了
    /// 「单击 Shift」之后，那时 Caps Lock 退化成只管大小写。
    pub fn refresh_mode_ui(&mut self) {
        let english = self.english_mode();
        self.indicator.update(english);
        let caps = modifiers::caps_lock_on();
        // 第一次只记不弹：启动或刚激活时不该凭空冒一个提示
        if self.last_osd_english.is_some_and(|last| last != english) {
            self.osd.show(if english { "英" } else { "中" });
        } else if self.last_osd_caps.is_some_and(|last| last != caps) {
            self.osd.show(if caps { "大写" } else { "小写" });
        }
        self.last_osd_english = Some(english);
        self.last_osd_caps = Some(caps);
    }

    /// 停用时忘掉已提示过的状态：下次激活的第一次刷新只记不弹，焦点来回挪不至于弹提示。
    pub fn forget_osd_state(&mut self) {
        self.last_osd_english = None;
        self.last_osd_caps = None;
    }

    /// 任一按键按下：除 Shift 自己以外的任何键（含别的修饰键）都作废待定的单击 Shift
    /// （Shift + 字母 / Shift + Cmd 组合都不该被当成单击）。
    pub fn note_key(&mut self, key_code: u16) {
        if !is_shift_key(key_code) {
            self.mode_switch.note_key();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::ModeSwitch;

    const LEFT_SHIFT: u16 = 56;

    fn enabled() -> ModeSwitch {
        let mut switch = ModeSwitch::default();
        switch.set_enabled(true);
        switch
    }

    #[test]
    fn a_bounce_right_after_switching_is_ignored() {
        let mut switch = enabled();
        let start = Instant::now();
        let at = |ms| start + Duration::from_millis(ms);
        assert!(!switch.shift_event_at(LEFT_SHIFT, true, at(0)));
        assert!(switch.shift_event_at(LEFT_SHIFT, false, at(80)));
        // 松开后 10 ms 补来的一对按下 / 抬起
        assert!(!switch.shift_event_at(LEFT_SHIFT, true, at(90)));
        assert!(!switch.shift_event_at(LEFT_SHIFT, false, at(95)));
        assert!(switch.english());
        // 冷却过后的下一次单击照常切回
        assert!(!switch.shift_event_at(LEFT_SHIFT, true, at(400)));
        assert!(switch.shift_event_at(LEFT_SHIFT, false, at(480)));
        assert!(!switch.english());
    }

    #[test]
    fn holding_shift_too_long_is_not_a_tap() {
        let mut switch = enabled();
        let start = Instant::now();
        assert!(!switch.shift_event_at(LEFT_SHIFT, true, start));
        assert!(!switch.shift_event_at(LEFT_SHIFT, false, start + Duration::from_millis(900)));
        assert!(!switch.english());
    }
}
