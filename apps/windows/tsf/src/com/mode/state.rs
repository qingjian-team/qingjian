//! 当前中 / 英模式与语言栏更新回调，文本服务与语言栏按钮（[`super::ModeButton`]）共享。

use core::cell::{Cell, RefCell};
use std::rc::Rc;

use windows::Win32::UI::TextServices::{ITfLangBarItemSink, TF_LBI_ICON, TF_LBI_STATUS};

use qingjian_platform::{SwitchKey, SwitchKeys, ThirdInputMethod};

/// 当前中英模式 + 语言栏更新回调，文本服务与语言栏按钮共享（STA 单线程）。
pub(crate) struct ModeState {
    /// `true` 是英文模式。
    english: Cell<bool>,

    /// 双击 Shift 切出的越南语模式。
    vietnamese: Cell<bool>,

    /// 中文单击 Shift 返回的上次外文：false 英文，true 越南语。
    last_foreign_vietnamese: Cell<bool>,

    /// 内置英文模式开关（`[general] english_mode`）：关掉后谁都不许切到英文。
    enabled: Cell<bool>,

    /// 勾着的中英切换键（`[shortcut] switch_mode`），单击判定与语言栏提示用。
    switch_keys: Cell<SwitchKeys>,

    third: Cell<ThirdInputMethod>,

    /// 系统登记进来的语言栏更新回调；由 [`super::ModeButton`] 的 `ITfSource` 登记 / 撤销。
    pub(super) sink: RefCell<Option<ITfLangBarItemSink>>,
}

impl ModeState {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self {
            english: Cell::new(false),
            vietnamese: Cell::new(false),
            last_foreign_vietnamese: Cell::new(false),
            enabled: Cell::new(true),
            switch_keys: Cell::new(SwitchKeys::default()),
            third: Cell::new(ThirdInputMethod::Off),
            sink: RefCell::new(None),
        })
    }

    pub(crate) fn english(&self) -> bool {
        self.english.get()
    }

    pub(crate) fn set_english(&self, english: bool) {
        self.english.set(english);
        if english {
            self.vietnamese.set(false);
            self.last_foreign_vietnamese.set(false);
        }
    }

    pub(crate) fn vietnamese(&self) -> bool {
        self.vietnamese.get()
    }

    /// 双击 Shift：英文 / 越南语互换；中文时切到“另一个外文”。
    pub(crate) fn switch_foreign(&self) {
        if self.english.get() {
            self.english.set(false);
            self.vietnamese.set(true);
            self.last_foreign_vietnamese.set(true);
        } else if self.vietnamese.get() {
            self.vietnamese.set(false);
            self.english.set(true);
            self.last_foreign_vietnamese.set(false);
        } else if self.last_foreign_vietnamese.get() {
            self.english.set(true);
            self.last_foreign_vietnamese.set(false);
        } else {
            self.vietnamese.set(true);
            self.last_foreign_vietnamese.set(true);
        }
    }

    /// 配置启用的单击切换键：中文 ↔ 上次外文。
    pub(crate) fn switch_chinese_foreign(&self) {
        if self.english.get() {
            self.last_foreign_vietnamese.set(false);
            self.english.set(false);
        } else if self.vietnamese.get() {
            self.last_foreign_vietnamese.set(true);
            self.vietnamese.set(false);
        } else if self.last_foreign_vietnamese.get() && self.third_enabled() {
            self.vietnamese.set(true);
        } else {
            self.english.set(true);
        }
    }

    /// 内置英文模式是否可用。
    pub(crate) fn enabled(&self) -> bool {
        self.enabled.get()
    }

    pub(crate) fn switch_keys(&self) -> SwitchKeys {
        self.switch_keys.get()
    }

    /// 单击切换键加上第三输入法需要监听的 Shift。
    pub(crate) fn tap_keys(&self) -> SwitchKeys {
        self.switch_keys
            .get()
            .with(SwitchKey::Shift, self.third_enabled())
    }

    /// 激活时按配置设一次。
    pub(crate) fn set_settings(
        &self,
        enabled: bool,
        switch_keys: SwitchKeys,
        third: ThirdInputMethod,
    ) {
        self.enabled.set(enabled);
        self.switch_keys.set(switch_keys);
        self.third.set(third);
        if third == ThirdInputMethod::Off {
            self.vietnamese.set(false);
        }
    }

    pub(crate) fn third_enabled(&self) -> bool {
        self.third.get() == ThirdInputMethod::Vietnamese
    }

    /// 通知系统重取图标 / 文字。
    pub(crate) fn notify(&self) {
        if let Some(sink) = self.sink.borrow().as_ref() {
            let _ = unsafe { sink.OnUpdate(TF_LBI_ICON | TF_LBI_STATUS) };
        }
    }
}
