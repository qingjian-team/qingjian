//! 「候选窗口」页：外观、排布、渲染引擎、字体（可搜索的列表）、拼音显示位置。

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSButton, NSPopUpButton};
use qingjian_platform::{
    CandidateRenderer, Config, LayoutMode, MAX_FONT_SIZE, MIN_FONT_SIZE, PreeditMode, ThemeMode,
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

    /// 青简渲染器 / 系统绘制。
    renderer: Retained<NSPopUpButton>,

    /// 候选窗字体：搜索框 + 列表。
    font: FontPicker,

    /// 候选词字号。
    font_size: Retained<NSPopUpButton>,

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
            "勾上后横排下 ↑ / ↓ 把一行展开成 6 行矩阵并换行，← / → 在候选之间移动（← 顶到第一个候选后接着按可移动拼音光标，任意位置也可用 ⌥← / ⌥→），第一排再按 ↑ 回到第一个候选，Esc 第一下先收回；不勾（缺省）按键与以前一样。",
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
        let size_titles: Vec<String> = (MIN_FONT_SIZE..=MAX_FONT_SIZE)
            .map(|size| size.to_string())
            .collect();
        let font_size = row_popup(layout, mtm, "字号", &size_titles, Setting::FontSize, target);
        note(
            layout,
            mtm,
            "候选词的字号（点），译文与序号按比例跟着缩放；只影响候选窗口（悬浮状态条不变），系统绘制退路下不生效。",
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
            renderer,
            font,
            font_size,
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
        select(
            &self.renderer,
            CandidateRenderer::ALL
                .iter()
                .position(|r| *r == general.renderer),
        );
        self.font.sync(&general.font);
        select(
            &self.font_size,
            Some(usize::from(
                general.font_size().saturating_sub(MIN_FONT_SIZE),
            )),
        );
        select(
            &self.preedit,
            PreeditMode::ALL.iter().position(|p| *p == general.preedit),
        );
    }
}
