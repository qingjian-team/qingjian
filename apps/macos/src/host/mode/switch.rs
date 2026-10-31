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

use crate::host::Host;

/// 只用单击 Shift 时沿用原来的防抖窗口；开了双击 Shift 后缩短，免得吞掉第二次按下。
const SINGLE_TAP_COOLDOWN: Duration = Duration::from_millis(150);

const DOUBLE_TAP_COOLDOWN: Duration = Duration::from_millis(50);

/// 按住超过这么久再松开不算单击：鼠标事件不进 IMK 会话，按住 Shift 拖选文字时没有别的键能作废这次单击。
const TAP_WINDOW: Duration = Duration::from_millis(300);

/// 两次完整的 Shift 单击落在这个窗口内，切换英文 / 第三输入法；超时才落实单击。
const DOUBLE_TAP_WINDOW: Duration = Duration::from_millis(400);

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

    /// 配置了双击 Shift 的第三输入法。
    third_enabled: bool,

    /// 当前处在第三输入法（越南语）。
    vietnamese: bool,

    /// 中文单击 Shift 返回的上次外文；false 英文，true 越南语。
    last_foreign_vietnamese: bool,

    /// 第一下完整 Shift 单击等待第二下；超时后才落实成单击，避免双击时先闪一次中间状态。
    pending_tap: Option<Instant>,
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

    pub fn vietnamese(&self) -> bool {
        self.vietnamese
    }

    pub fn set_third_enabled(&mut self, enabled: bool) {
        self.third_enabled = enabled;
        if !enabled {
            self.vietnamese = false;
            self.pending_tap = None;
        }
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
            self.vietnamese = false;
            self.pending_tap = None;
        }
    }

    /// 焦点切换 / 停用：忘掉这次还没抬起的 Shift，
    /// 免得「按着 Shift 切走再切回、然后松开」被当成一次单击。
    pub fn reset(&mut self) {
        self.pending = None;
        self.pending_tap = None;
    }

    /// 任一普通按键按下：Shift 按下到抬起之间插了别的键，这次不算单击。
    pub fn note_key(&mut self) {
        self.pending = None;
    }

    pub fn has_pending_tap(&self) -> bool {
        self.pending_tap.is_some()
    }

    /// 400 ms 内没有第二次 Shift：勾了单击 Shift 才落实为“中文 ↔ 上次外文”。
    pub fn resolve_pending(&mut self) -> bool {
        self.resolve_pending_at(Instant::now())
    }

    fn resolve_pending_at(&mut self, now: Instant) -> bool {
        if self.pending.is_some() {
            return false;
        }
        if self
            .pending_tap
            .is_none_or(|at| now.duration_since(at) < DOUBLE_TAP_WINDOW)
        {
            return false;
        }
        self.pending_tap = None;
        if !self.enabled {
            return false;
        }
        self.single_switch();
        true
    }

    fn single_switch(&mut self) {
        if self.english {
            self.last_foreign_vietnamese = false;
            self.english = false;
        } else if self.vietnamese {
            self.last_foreign_vietnamese = true;
            self.vietnamese = false;
        } else if self.last_foreign_vietnamese && self.third_enabled {
            self.vietnamese = true;
        } else {
            self.english = true;
        }
    }

    fn double_switch(&mut self) {
        if self.english {
            self.english = false;
            self.vietnamese = true;
            self.last_foreign_vietnamese = true;
        } else if self.vietnamese {
            self.vietnamese = false;
            self.english = true;
            self.last_foreign_vietnamese = false;
        } else if self.last_foreign_vietnamese {
            self.english = true;
            self.last_foreign_vietnamese = false;
        } else {
            self.vietnamese = true;
            self.last_foreign_vietnamese = true;
        }
    }

    /// 修饰键事件：`key_code` 那次按下（`down`）。构成一次「单击 Shift」返回 `true`。
    pub fn shift_event(&mut self, key_code: u16, down: bool) -> bool {
        self.shift_event_at(key_code, down, Instant::now())
    }

    fn shift_event_at(&mut self, key_code: u16, down: bool, now: Instant) -> bool {
        if !self.enabled && !self.third_enabled {
            return false;
        }
        // 别的修饰键（Cmd / Ctrl / Option / Caps Lock）：这次不算单击
        if !is_shift_key(key_code) {
            self.pending = None;
            return false;
        }
        let cooldown = if self.third_enabled {
            DOUBLE_TAP_COOLDOWN
        } else {
            SINGLE_TAP_COOLDOWN
        };
        if self
            .toggled_at
            .is_some_and(|at| now.duration_since(at) < cooldown)
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
            if !self.third_enabled {
                if self.enabled {
                    self.single_switch();
                }
            } else if self
                .pending_tap
                .is_some_and(|at| now.duration_since(at) <= DOUBLE_TAP_WINDOW)
            {
                self.pending_tap = None;
                self.double_switch();
            } else {
                self.pending_tap = Some(now);
            }
            self.toggled_at = Some(now);
        }
        clicked && (!self.third_enabled || self.pending_tap.is_none())
    }
}

impl Host {
    /// 定时器处理 400 ms 内没有第二下的 Shift；没勾单击 Shift 时只清掉双击等待。
    pub fn resolve_pending_shift(&mut self) -> bool {
        let switched = self.mode_switch.resolve_pending();
        self.indicator
            .set_fast_poll(self.mode_switch.has_pending_tap());
        if !switched {
            return false;
        }
        tracing::info!(
            english = self.english_mode(),
            vietnamese = self.vietnamese_mode(),
            "Shift 单击切换输入法"
        );
        self.refresh_mode_ui();
        self.render();
        true
    }

    /// 中英模式：勾了「单击 Shift」时由 Shift 的切换决定（Caps Lock 退化成只管大小写），没勾看 Caps Lock。
    pub fn english_mode(&self) -> bool {
        if self.mode_switch.vietnamese() {
            return false;
        }
        if self.mode_switch.enabled() {
            self.mode_switch.english()
        } else {
            modifiers::caps_lock_on()
        }
    }

    pub fn vietnamese_mode(&self) -> bool {
        self.mode_switch.vietnamese()
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
        let switched = self.mode_switch.shift_event(key_code, down);
        self.indicator
            .set_fast_poll(self.mode_switch.has_pending_tap());
        if !switched {
            return false;
        }
        tracing::info!(
            english = self.english_mode(),
            vietnamese = self.vietnamese_mode(),
            "Shift 切换输入法"
        );
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
        let vietnamese = self.vietnamese_mode();
        self.indicator.update(english, vietnamese);
        let caps = modifiers::caps_lock_on();
        // 第一次只记不弹：启动或刚激活时不该凭空冒一个提示
        if self
            .last_osd_vietnamese
            .is_some_and(|last| last != vietnamese)
        {
            self.osd.show(if vietnamese {
                "越"
            } else if english {
                "英"
            } else {
                "中"
            });
        } else if self.last_osd_english.is_some_and(|last| last != english) {
            self.osd.show(if english { "英" } else { "中" });
        } else if self.last_osd_caps.is_some_and(|last| last != caps) {
            self.osd.show(if caps { "大写" } else { "小写" });
        }
        self.last_osd_english = Some(english);
        self.last_osd_vietnamese = Some(vietnamese);
        self.last_osd_caps = Some(caps);
    }

    /// 停用时忘掉已提示过的状态：下次激活的第一次刷新只记不弹，焦点来回挪不至于弹提示。
    pub fn forget_osd_state(&mut self) {
        self.last_osd_english = None;
        self.last_osd_vietnamese = None;
        self.last_osd_caps = None;
    }

    /// 任一按键按下：除 Shift 自己以外的任何键（含别的修饰键）都作废待定的单击 Shift
    /// （Shift + 字母 / Shift + Cmd 组合都不该被当成单击）。
    pub fn note_key(&mut self, key_code: u16) -> bool {
        if !is_shift_key(key_code) {
            // 用户不等双击窗口就开始打字：勾了单击 Shift 才先切模式，再处理这个键。
            let switched = if self.mode_switch.has_pending_tap() && self.mode_switch.enabled() {
                self.mode_switch.pending_tap = None;
                self.mode_switch.single_switch();
                self.refresh_mode_ui();
                true
            } else {
                self.mode_switch.pending_tap = None;
                false
            };
            self.indicator.set_fast_poll(false);
            self.mode_switch.note_key();
            return switched;
        }
        false
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

    #[test]
    fn double_shift_toggles_the_third_input_method() {
        let mut switch = enabled();
        switch.set_third_enabled(true);
        let start = Instant::now();
        let at = |ms| start + Duration::from_millis(ms);
        assert!(!switch.shift_event_at(LEFT_SHIFT, true, at(0)));
        assert!(!switch.shift_event_at(LEFT_SHIFT, false, at(40)));
        assert!(!switch.english(), "第一下待定，不先闪英文");
        assert!(!switch.shift_event_at(LEFT_SHIFT, true, at(90)));
        assert!(switch.shift_event_at(LEFT_SHIFT, false, at(120)));
        assert!(switch.vietnamese());
        assert!(!switch.english());
        assert!(!switch.shift_event_at(LEFT_SHIFT, true, at(220)));
        assert!(!switch.shift_event_at(LEFT_SHIFT, false, at(250)));
        assert!(!switch.shift_event_at(LEFT_SHIFT, true, at(300)));
        assert!(switch.shift_event_at(LEFT_SHIFT, false, at(330)));
        assert!(switch.english());
        assert!(!switch.vietnamese());
    }

    #[test]
    fn double_shift_works_without_single_shift_switching() {
        let mut switch = ModeSwitch::default();
        switch.set_third_enabled(true);
        let start = Instant::now();
        let at = |ms| start + Duration::from_millis(ms);
        assert!(!switch.shift_event_at(LEFT_SHIFT, true, at(0)));
        assert!(!switch.shift_event_at(LEFT_SHIFT, false, at(40)));
        assert!(!switch.english());
        assert!(!switch.shift_event_at(LEFT_SHIFT, true, at(90)));
        assert!(switch.shift_event_at(LEFT_SHIFT, false, at(120)));
        assert!(switch.vietnamese());
    }

    #[test]
    fn a_fast_human_double_tap_is_not_mistaken_for_keyboard_bounce() {
        let mut switch = enabled();
        switch.set_third_enabled(true);
        let start = Instant::now();
        let at = |ms| start + Duration::from_millis(ms);
        assert!(!switch.shift_event_at(LEFT_SHIFT, true, at(0)));
        assert!(!switch.shift_event_at(LEFT_SHIFT, false, at(30)));
        assert!(!switch.shift_event_at(LEFT_SHIFT, true, at(80)));
        assert!(switch.shift_event_at(LEFT_SHIFT, false, at(110)));
        assert!(switch.vietnamese());
    }

    #[test]
    fn cooldown_is_150ms_without_double_shift_and_50ms_with_it() {
        let start = Instant::now();
        let at = |ms| start + Duration::from_millis(ms);

        let mut single = enabled();
        assert!(!single.shift_event_at(LEFT_SHIFT, true, at(0)));
        assert!(single.shift_event_at(LEFT_SHIFT, false, at(60)));
        // 距上次切换 100 ms：没开双击时仍按原来的 150 ms 防抖丢掉。
        assert!(!single.shift_event_at(LEFT_SHIFT, true, at(160)));
        assert!(!single.shift_event_at(LEFT_SHIFT, false, at(210)));
        assert!(single.english());

        let mut double = enabled();
        double.set_third_enabled(true);
        assert!(!double.shift_event_at(LEFT_SHIFT, true, at(0)));
        assert!(!double.shift_event_at(LEFT_SHIFT, false, at(40)));
        // 距上次边沿超过 50 ms，且第二次抬起仍在首个抬起后的 400 ms 内。
        assert!(!double.shift_event_at(LEFT_SHIFT, true, at(100)));
        assert!(double.shift_event_at(LEFT_SHIFT, false, at(130)));
        assert!(double.vietnamese());
    }

    #[test]
    fn single_shift_returns_to_the_last_foreign_language() {
        let mut switch = enabled();
        switch.set_third_enabled(true);
        let start = Instant::now();
        let at = |ms| start + Duration::from_millis(ms);

        assert!(!switch.shift_event_at(LEFT_SHIFT, true, at(0)));
        assert!(!switch.shift_event_at(LEFT_SHIFT, false, at(30)));
        assert!(!switch.resolve_pending_at(at(429)));
        assert!(switch.resolve_pending_at(at(430)));
        assert!(switch.english());

        assert!(!switch.shift_event_at(LEFT_SHIFT, true, at(550)));
        assert!(!switch.shift_event_at(LEFT_SHIFT, false, at(580)));
        assert!(switch.resolve_pending_at(at(980)));
        assert!(!switch.english());

        // 中文双击换到另一个外文（越南语），不经过英文中间态。
        assert!(!switch.shift_event_at(LEFT_SHIFT, true, at(1100)));
        assert!(!switch.shift_event_at(LEFT_SHIFT, false, at(1130)));
        assert!(!switch.shift_event_at(LEFT_SHIFT, true, at(1380)));
        assert!(switch.shift_event_at(LEFT_SHIFT, false, at(1410)));
        assert!(switch.vietnamese());

        assert!(!switch.shift_event_at(LEFT_SHIFT, true, at(1520)));
        assert!(!switch.shift_event_at(LEFT_SHIFT, false, at(1550)));
        assert!(switch.resolve_pending_at(at(1950)));
        assert!(!switch.vietnamese());
        assert!(!switch.shift_event_at(LEFT_SHIFT, true, at(2070)));
        assert!(!switch.shift_event_at(LEFT_SHIFT, false, at(2100)));
        assert!(switch.resolve_pending_at(at(2500)));
        assert!(switch.vietnamese(), "中文单击回到上次外文越南语");
    }

    #[test]
    fn shift_single_does_nothing_when_only_double_shift_is_enabled() {
        let mut switch = ModeSwitch::default();
        switch.set_third_enabled(true);
        let start = Instant::now();
        let at = |ms| start + Duration::from_millis(ms);

        assert!(!switch.shift_event_at(LEFT_SHIFT, true, at(0)));
        assert!(!switch.shift_event_at(LEFT_SHIFT, false, at(30)));
        assert!(!switch.resolve_pending_at(at(430)));
        assert!(!switch.english());
        assert!(!switch.vietnamese());
    }
}
