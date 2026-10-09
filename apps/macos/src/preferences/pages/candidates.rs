//! 「候选窗口」页：外观、主题、排布、渲染引擎、字体（可搜索的列表）与字号、过渡动画、拼音显示位置。

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSButton, NSPopUpButton, NSTextField};
use objc2_foundation::NSString;
use qingjian_platform::{Appearance, CandidateRenderer, Config, FontSize, LayoutMode, PreeditMode};
use qingjian_render::ThemeLibrary;

use crate::candidates::available_families;
use crate::preferences::controls::{
    button, checkbox, note, row_checkbox, row_control, row_popup, select, set_checked, set_items,
    text_field,
};
use crate::preferences::font_picker::FontPicker;
use crate::preferences::layout::{CONTROL_X, Layout, ROW_HEIGHT};
use crate::preferences::setting::Setting;
use crate::preferences::target::PreferencesTarget;

pub struct CandidatesPage {
    /// 外观：跟随系统 / 浅色 / 深色。
    appearance: Retained<NSPopUpButton>,

    /// 主题：内置主题按显示名列出。
    theme: Retained<NSPopUpButton>,

    /// 竖排 / 横排。
    layout_mode: Retained<NSPopUpButton>,

    /// 横排时上 / 下键展开成多行矩阵。
    horizontal_grid: Retained<NSButton>,

    /// 青简渲染器 / 系统绘制。
    renderer: Retained<NSPopUpButton>,

    /// 候选窗字体：搜索框 + 列表。
    font: FontPicker,

    /// 候选字字号，空为用主题的。
    candidate_size: Retained<NSTextField>,

    /// 译文字号，空为用主题的。
    annotation_size: Retained<NSTextField>,

    /// 过渡动画。
    animations: Retained<NSButton>,

    /// 拼音显示位置。
    preedit: Retained<NSPopUpButton>,
}

impl CandidatesPage {
    pub fn build(layout: &mut Layout, mtm: MainThreadMarker, target: &PreferencesTarget) -> Self {
        let appearance_titles: Vec<String> = Appearance::ALL
            .iter()
            .map(|a| a.label().to_owned())
            .collect();
        let appearance = row_popup(
            layout,
            mtm,
            "外观",
            &appearance_titles,
            Setting::Appearance,
            target,
        );
        let theme = row_popup(
            layout,
            mtm,
            "主题",
            &theme_titles(&themes()),
            Setting::Theme,
            target,
        );
        note(
            layout,
            mtm,
            "主题只对青简渲染器生效；樱花只有浅色，其余主题按上面的外观切换浅色与深色。",
        );
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
        let candidate_size =
            size_field(layout, mtm, "候选字号", Setting::CandidateFontSize, target);
        let annotation_size =
            size_field(layout, mtm, "译文字号", Setting::AnnotationFontSize, target);
        note(
            layout,
            mtm,
            "单位是点，按回车生效；清空或填 0 回到主题的字号。行高跟着候选字号缩放。只对青简渲染器生效。",
        );
        let reset = button(mtm, "恢复主题字号", Setting::ResetFontSizes, target);
        layout.place(&reset, CONTROL_X, 140.0, ROW_HEIGHT + 4.0);
        layout.next_row(ROW_HEIGHT + 4.0);
        let animations = checkbox(mtm, "过渡动画", Setting::Animations, target);
        row_checkbox(layout, &animations);
        note(
            layout,
            mtm,
            "高亮换候选时滑过去、主题里的循环动画。关掉后直接跳到位；系统打开了「减弱动态效果」时也不播。",
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
            appearance,
            theme,
            layout_mode,
            horizontal_grid,
            renderer,
            font,
            candidate_size,
            annotation_size,
            animations,
            preedit,
        }
    }

    pub fn sync(&self, config: &Config) {
        let general = &config.general;
        select(
            &self.appearance,
            Appearance::ALL
                .iter()
                .position(|a| *a == general.appearance()),
        );
        // 主题目录里可能新放了主题，每次同步都重列
        let themes = themes();
        set_items(&self.theme, &theme_titles(&themes));
        select(
            &self.theme,
            themes
                .themes()
                .iter()
                .position(|t| t.id() == general.theme_id()),
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
        let theme = themes.resolve(general.theme_id(), false);
        set_size(
            &self.candidate_size,
            general.candidate_font_size,
            theme.theme_size("candidate"),
        );
        set_size(
            &self.annotation_size,
            general.annotation_font_size,
            theme.theme_size("annotation"),
        );
        set_checked(&self.animations, general.animations);
        select(
            &self.preedit,
            PreeditMode::ALL.iter().position(|p| *p == general.preedit),
        );
    }
}

/// 一行字号文本框。
fn size_field(
    layout: &mut Layout,
    mtm: MainThreadMarker,
    title: &str,
    setting: Setting,
    target: &PreferencesTarget,
) -> Retained<NSTextField> {
    let field = text_field(mtm, setting, target);
    row_control(layout, mtm, title, &field);
    field
}

/// 设过的字号，没设过显示主题的（用户知道从哪个数开始调）；整数不带小数点。
fn set_size(field: &NSTextField, size: FontSize, theme: Option<f32>) {
    let text = size
        .get()
        .or(theme)
        .map_or_else(String::new, |size| size.to_string());
    field.setStringValue(&NSString::from_str(&text));
}

/// 主题库：内置主题加用户主题目录里的。
fn themes() -> ThemeLibrary {
    ThemeLibrary::load(crate::app::paths::themes_dir().as_deref())
}

/// 主题下拉的选项：显示名。
fn theme_titles(themes: &ThemeLibrary) -> Vec<String> {
    themes
        .themes()
        .iter()
        .map(|theme| theme.name().to_owned())
        .collect()
}
