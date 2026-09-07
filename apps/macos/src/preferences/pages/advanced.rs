//! 「高级」页：打开配置文件、详细日志、输入日志。

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::NSButton;
use qingjian_platform::{Config, LogLevel};

use crate::preferences::controls::{
    GROUP_GAP, button, checkbox, note_full, row_checkbox, set_checked,
};
use crate::preferences::layout::{Layout, PAGE_PADDING, ROW_HEIGHT};
use crate::preferences::setting::Setting;
use crate::preferences::target::PreferencesTarget;

pub struct AdvancedPage {
    /// 详细日志（debug 级）。
    verbose: Retained<NSButton>,

    /// 记录输入日志。
    input_log: Retained<NSButton>,
}

impl AdvancedPage {
    pub fn build(layout: &mut Layout, mtm: MainThreadMarker, target: &PreferencesTarget) -> Self {
        let open = button(
            mtm,
            "在编辑器中打开配置文件",
            Setting::OpenConfigFile,
            target,
        );
        layout.place(&open, PAGE_PADDING, 220.0, ROW_HEIGHT + 4.0);
        layout.next_row(ROW_HEIGHT + 4.0);
        note_full(
            layout,
            mtm,
            "这里的每一项设置都对应配置文件里的一行，手改文件保存后立即生效；文件里有更多注释与少数不常用的选项。",
        );
        layout.space(GROUP_GAP);
        let verbose = checkbox(mtm, "详细日志", Setting::VerboseLog, target);
        row_checkbox(layout, &verbose);
        note_full(
            layout,
            mtm,
            "会把敲的拼音与上屏的文字记进日志，只在配合作者排查问题时打开，查完关掉。日志在「关于」页可以打开。",
        );
        layout.space(GROUP_GAP);
        let input_log = checkbox(mtm, "记录输入日志", Setting::InputLog, target);
        row_checkbox(layout, &input_log);
        note_full(
            layout,
            mtm,
            "每次上屏记一行：敲的键、看到的候选、选了哪个。只写在这台电脑的数据目录里，不上传；用来离线评测排序、训练个人模型。",
        );
        let clear = button(mtm, "清空输入日志", Setting::ClearInputLog, target);
        layout.place(&clear, PAGE_PADDING, 160.0, ROW_HEIGHT + 4.0);
        layout.next_row(ROW_HEIGHT + 4.0);
        Self { verbose, input_log }
    }

    pub fn sync(&self, config: &Config) {
        set_checked(&self.verbose, config.general.log_level == LogLevel::Debug);
        set_checked(&self.input_log, config.general.input_log);
    }
}
