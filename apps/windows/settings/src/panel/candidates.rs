//! 「候选窗口」设置页：外观、排布、拼音显示位置（对齐 macOS「候选窗口」页）。

use qingjian_platform::{LayoutMode, PreeditMode, ThemeMode};
use windows_reactor::*;

use super::{Message, Settings, field, page};

pub(super) fn view(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let g = &settings.config.general;
    let rows = [
        field(
            "外观",
            "",
            ComboBox::new()
                .items_source(ThemeMode::ALL.iter().map(|mode| mode.label()))
                .selected_index(
                    ThemeMode::ALL
                        .iter()
                        .position(|mode| *mode == g.theme)
                        .unwrap_or(0),
                )
                .on_selection_changed(context.callback(Message::Theme)),
        ),
        field(
            "排布",
            "横排时只给高亮的候选显示译词。",
            ComboBox::new()
                .items_source(LayoutMode::ALL.iter().map(|mode| mode.label()))
                .selected_index(
                    LayoutMode::ALL
                        .iter()
                        .position(|mode| *mode == g.layout)
                        .unwrap_or(0),
                )
                .on_selection_changed(context.callback(Message::Layout)),
        ),
        field(
            "拼音显示",
            "「只在候选窗口」时正在敲的拼音不显示在应用里，终端或行内拼音不正常的应用可以选它。",
            ComboBox::new()
                .items_source(PreeditMode::ALL.iter().map(|mode| mode.label()))
                .selected_index(
                    PreeditMode::ALL
                        .iter()
                        .position(|mode| *mode == g.preedit)
                        .unwrap_or(0),
                )
                .on_selection_changed(context.callback(Message::Preedit)),
        ),
    ];
    page("候选窗口", StackPanel::new().spacing(16.0).children(rows))
}
