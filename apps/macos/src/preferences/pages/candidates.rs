//! 「候选窗口」页：外观、排布、渲染引擎、字体（可搜索的列表）、拼音显示位置。

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSButton, NSPopUpButton};
use qingjian_platform::{
    CandidateRenderer, Config, LayoutMode, PreeditMode, ThemeMode, VerticalAboveArrowKeys,
};

use crate::candidates::available_families;
use crate::preferences::controls::{checkbox, note, row_checkbox, row_popup, select, set_checked};
use crate::preferences::font_picker::FontPicker;
use crate::preferences::layout::Layout;
use crate::preferences::setting::Setting;
use crate::preferences::target::PreferencesTarget;

pub struct CandidatesPage {
    /// 外观：跟随系统 / 浅色 / 深色。
    theme: Retained<NSPopUpButton>,

    /// 竖排 / 横排。
    layout_mode: Retained<NSPopUpButton>,

    /// 横排时上 / 下键展开成多行矩阵。
    horizontal_grid: Retained<NSButton>,

    /// 竖排候选窗位于输入行上方时倒序显示。
    vertical_above_reverse: Retained<NSButton>,

    /// 倒序时按候选顺序或屏幕方向移动高亮。
    vertical_above_arrow_keys: Retained<NSPopUpButton>,

    /// 青简渲染器 / 系统绘制。
    renderer: Retained<NSPopUpButton>,

    /// 候选窗字体：搜索框 + 列表。
    font: FontPicker,

    /// 拼音显示位置。
    preedit: Retained<NSPopUpButton>,
}

impl CandidatesPage {
    pub fn build(layout: &mut Layout, mtm: MainThreadMarker, target: &PreferencesTarget) -> Self {
        let theme_titles: Vec<String> = ThemeMode::ALL
            .iter()
            .map(|t| t.label().to_owned())
            .collect();
        let theme = row_popup(layout, mtm, "外观", &theme_titles, Setting::Theme, target);
        let layout_titles: Vec<String> = LayoutMode::ALL
            .iter()
            .map(|l| l.label().to_owned())
            .collect();
        let layout_mode = row_popup(layout, mtm, "排布", &layout_titles, Setting::Layout, target);
        note(layout, mtm, "横排时只给高亮的候选显示译词。");
        let horizontal_grid = checkbox(
            mtm,
            "横排时 ↑ / ↓ 展开成多行",
            Setting::HorizontalGrid,
            target,
        );
        row_checkbox(layout, &horizontal_grid);
        note(
            layout,
            mtm,
            "勾上后横排下 ↑ / ↓ 把一行展开成 6 行矩阵并换行，← / → 在候选之间移动（拼音光标用 ⌥← / ⌥→），Esc 第一下先收回；不勾（缺省）按键与以前一样。",
        );
        let vertical_above_reverse = checkbox(
            mtm,
            "上方竖排候选倒序显示",
            Setting::VerticalAboveReverse,
            target,
        );
        row_checkbox(layout, &vertical_above_reverse);
        let arrow_titles: Vec<String> = VerticalAboveArrowKeys::ALL
            .iter()
            .map(|keys| keys.label().to_owned())
            .collect();
        let vertical_above_arrow_keys = row_popup(
            layout,
            mtm,
            "倒序时方向键",
            &arrow_titles,
            Setting::VerticalAboveArrowKeys,
            target,
        );
        note(
            layout,
            mtm,
            "倒序时拼音在下、页码在上，首项靠近输入行，数字键仍按编号选词。按候选顺序：↓ 选下一项；按屏幕方向：↑ 向上、↓ 向下。",
        );
        let renderer_titles: Vec<String> = CandidateRenderer::ALL
            .iter()
            .map(|r| r.label().to_owned())
            .collect();
        let renderer = row_popup(
            layout,
            mtm,
            "渲染引擎",
            &renderer_titles,
            Setting::Renderer,
            target,
        );
        note(layout, mtm, "青简渲染器让候选窗口在各平台一致。");
        let font = FontPicker::build(layout, mtm, "字体", available_families(mtm));
        note(
            layout,
            mtm,
            "只对青简渲染器生效；没装的字体自动回到系统字体。",
        );
        let preedit_titles: Vec<String> = PreeditMode::ALL
            .iter()
            .map(|p| p.label().to_owned())
            .collect();
        let preedit = row_popup(
            layout,
            mtm,
            "拼音显示",
            &preedit_titles,
            Setting::Preedit,
            target,
        );
        note(
            layout,
            mtm,
            "「只在候选窗口」时正在敲的拼音不显示在应用里，终端或行内拼音显示不正常的应用可以选它。",
        );
        Self {
            theme,
            layout_mode,
            horizontal_grid,
            vertical_above_reverse,
            vertical_above_arrow_keys,
            renderer,
            font,
            preedit,
        }
    }

    pub fn sync(&self, config: &Config) {
        let general = &config.general;
        select(
            &self.theme,
            ThemeMode::ALL.iter().position(|t| *t == general.theme),
        );
        select(
            &self.layout_mode,
            LayoutMode::ALL.iter().position(|l| *l == general.layout),
        );
        set_checked(&self.horizontal_grid, general.horizontal_grid);
        self.horizontal_grid
            .setEnabled(general.layout == LayoutMode::Horizontal);
        set_checked(&self.vertical_above_reverse, general.vertical_above_reverse);
        self.vertical_above_reverse
            .setEnabled(general.layout == LayoutMode::Vertical);
        select(
            &self.vertical_above_arrow_keys,
            VerticalAboveArrowKeys::ALL
                .iter()
                .position(|keys| *keys == general.vertical_above_arrow_keys),
        );
        self.vertical_above_arrow_keys
            .setEnabled(general.layout == LayoutMode::Vertical && general.vertical_above_reverse);
        select(
            &self.renderer,
            CandidateRenderer::ALL
                .iter()
                .position(|r| *r == general.renderer),
        );
        self.font.sync(&general.font);
        select(
            &self.preedit,
            PreeditMode::ALL.iter().position(|p| *p == general.preedit),
        );
    }
}
