//! 原生快捷键录制器，支持组合键与中英切换专用的单击 Shift。

use std::cell::{Cell, RefCell};

use crate::imk::ShiftTap;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSBezelStyle, NSButton, NSEvent, NSEventModifierFlags};
use objc2_foundation::{NSObjectProtocol, NSRect, NSString};
use qingjian_platform::{KeyCombo, Modifiers};

/// 录制中的按钮标题。
const RECORDING_TITLE: &str = "按下新的快捷键…";

/// Esc 的键码：取消录制。
const ESCAPE_KEY: u16 = 53;

/// 录制器的状态。
pub struct Ivars {
    /// 只记修饰键（配数字键上屏译词那两项）：按住修饰键再按任意键，键本身不算。
    modifiers_only: bool,

    /// 中英切换控件额外允许录制单击 Shift。
    allow_shift: Cell<bool>,

    /// 与实际输入控制器使用同一个手势判定，防止录制与执行不一致。
    shift_tap: RefCell<ShiftTap>,

    /// 正在等用户按键。
    recording: Cell<bool>,

    /// 当前值的配置写法（`control+option+t` / `shift+option`），`changed:` 里由 `setting_from_sender` 取走。
    recorded: RefCell<String>,

    /// 当前值给人看的写法，取消录制时恢复到标题上。
    label: RefCell<String>,
}

define_class!(
    // SAFETY: NSButton 允许子类化；没有实现 Drop。
    #[unsafe(super(NSButton))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    /// 快捷键录制按钮：点一下进入录制，按下组合键就记下并发 `changed:`，Esc 或失焦取消。
    /// 标题显示当前组合（`⌃⌥T` / `⇧⌥`），配置写法放在 ivar 里。
    pub struct KeyRecorder;

    impl KeyRecorder {
        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool {
            true
        }

        /// 点击：进入录制，成为第一响应者。不调父类，免得当成普通按钮点击发 action。
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, _event: &NSEvent) {
            if let Some(window) = self.window() {
                window.makeFirstResponder(Some(self));
            }
            self.ivars().recording.set(true);
            self.ivars().shift_tap.borrow_mut().reset();
            self.setTitle(&NSString::from_str(RECORDING_TITLE));
        }

        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) {
            self.ivars().shift_tap.borrow_mut().cancel();
            if !self.ivars().recording.get() {
                // SAFETY: 不在录制中就按普通按钮处理
                let _: () = unsafe { msg_send![super(self), keyDown: event] };
                return;
            }
            if event.keyCode() == ESCAPE_KEY {
                self.cancel();
                return;
            }
            let flags = event.modifierFlags();
            let modifiers = Modifiers {
                option: flags.contains(NSEventModifierFlags::Option),
                shift: flags.contains(NSEventModifierFlags::Shift),
                control: flags.contains(NSEventModifierFlags::Control),
                command: flags.contains(NSEventModifierFlags::Command),
            };
            // 没按修饰键的单键不算快捷键，继续等
            if modifiers.is_empty() {
                return;
            }
            let value = if self.ivars().modifiers_only {
                Some((modifiers.key(), modifiers.label()))
            } else {
                event
                    .charactersIgnoringModifiers()
                    .and_then(|c| c.to_string().chars().next())
                    .filter(|c| c.is_ascii_alphanumeric())
                    .map(|c| {
                        let combo = KeyCombo {
                            modifiers,
                            key: c.to_ascii_lowercase(),
                        };
                        (combo.key_string(), combo.label())
                    })
            };
            let Some((key, label)) = value else {
                return;
            };
            self.finish(key, label);
        }

        #[unsafe(method(flagsChanged:))]
        fn flags_changed(&self, event: &NSEvent) {
            if self.ivars().recording.get() && self.ivars().allow_shift.get() {
                let tapped = self.ivars().shift_tap.borrow_mut().flags_changed(
                    event.keyCode(), event.modifierFlags(), event.timestamp(),
                );
                if tapped {
                    let value = qingjian_platform::ModeSwitch::Shift;
                    self.finish(value.key_string(), value.label());
                }
            }
        }

        /// 失焦：录制没完成就恢复原标题。
        #[unsafe(method(resignFirstResponder))]
        fn resign_first_responder(&self) -> bool {
            if self.ivars().recording.get() {
                self.cancel();
            }
            // SAFETY: 父类的缺省实现
            unsafe { msg_send![super(self), resignFirstResponder] }
        }
    }

    unsafe impl NSObjectProtocol for KeyRecorder {}
);

impl KeyRecorder {
    /// `modifiers_only` 为真时只记修饰键组合。
    pub fn new(mtm: MainThreadMarker, modifiers_only: bool) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(Ivars {
            modifiers_only,
            allow_shift: Cell::new(false),
            shift_tap: RefCell::new(ShiftTap::default()),
            recording: Cell::new(false),
            recorded: RefCell::new(String::new()),
            label: RefCell::new(String::new()),
        });
        let this: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: NSRect::ZERO] };
        this.setBezelStyle(NSBezelStyle::Push);
        this
    }

    /// 按配置刷新显示：`key` 是配置写法，`label` 是给人看的。
    pub fn show(&self, key: &str, label: &str) {
        if self.ivars().recording.get() {
            return;
        }
        *self.ivars().recorded.borrow_mut() = key.to_owned();
        *self.ivars().label.borrow_mut() = label.to_owned();
        self.setTitle(&NSString::from_str(label));
    }

    /// 最近一次录到的值的配置写法。
    pub fn recorded(&self) -> String {
        self.ivars().recorded.borrow().clone()
    }

    /// 仅中英切换项开启，其他快捷键仍遵循各自原有的组合规则。
    pub fn enable_shift(&self) {
        self.ivars().allow_shift.set(true);
    }

    fn finish(&self, key: String, label: String) {
        self.ivars().recording.set(false);
        self.ivars().shift_tap.borrow_mut().reset();
        *self.ivars().recorded.borrow_mut() = key;
        *self.ivars().label.borrow_mut() = label.clone();
        self.setTitle(&NSString::from_str(&label));
        if let Some(window) = self.window() {
            window.makeFirstResponder(None);
        }
        let target: Option<Retained<AnyObject>> = self.target();
        // SAFETY: 选择器是 PreferencesTarget 的 changed:，签名 (id) -> void。
        unsafe {
            self.sendAction_to(self.action(), target.as_deref());
        }
    }

    fn cancel(&self) {
        self.ivars().shift_tap.borrow_mut().reset();
        self.ivars().recording.set(false);
        let label = self.ivars().label.borrow().clone();
        self.setTitle(&NSString::from_str(&label));
    }
}
