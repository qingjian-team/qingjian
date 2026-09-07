//! 「模糊音」页：九条规则的勾选框，三列排。

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::NSButton;
use qingjian_core::FuzzyRules;
use qingjian_platform::Config;

use crate::preferences::controls::{checkbox, note_full, set_checked};
use crate::preferences::layout::{Layout, PAGE_PADDING, ROW_HEIGHT};
use crate::preferences::setting::Setting;
use crate::preferences::target::PreferencesTarget;

/// 勾选框每行几个。
const FUZZY_COLUMNS: usize = 3;

/// 勾选框的列距。
const FUZZY_COLUMN_WIDTH: f64 = 150.0;

pub struct FuzzyPage {
    /// 每条规则一个勾选框，顺序同 `FuzzyRules::NAMES`。
    buttons: Vec<Retained<NSButton>>,
}

impl FuzzyPage {
    pub fn build(layout: &mut Layout, mtm: MainThreadMarker, target: &PreferencesTarget) -> Self {
        let mut buttons = Vec::with_capacity(FuzzyRules::NAMES.len());
        for (index, name) in FuzzyRules::NAMES.iter().enumerate() {
            let column = index % FUZZY_COLUMNS;
            if column == 0 && index > 0 {
                layout.next_row(ROW_HEIGHT);
            }
            let button = checkbox(
                mtm,
                &name.replace('_', " = "),
                Setting::Fuzzy(index),
                target,
            );
            layout.place(
                &button,
                PAGE_PADDING + column as f64 * FUZZY_COLUMN_WIDTH,
                FUZZY_COLUMN_WIDTH - 6.0,
                ROW_HEIGHT,
            );
            buttons.push(button);
        }
        layout.next_row(ROW_HEIGHT);
        note_full(
            layout,
            mtm,
            "勾上的两种读音互相通用（比如勾了 z = zh，敲 zi 也出 zhi 的字），模糊命中的词排在准确命中之后。",
        );
        Self { buttons }
    }

    pub fn sync(&self, config: &Config) {
        for (button, name) in self.buttons.iter().zip(FuzzyRules::NAMES) {
            set_checked(button, config.fuzzy.is_on(name));
        }
    }
}
