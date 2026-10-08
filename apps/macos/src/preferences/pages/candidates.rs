//! 「候选窗口」页：外观、排布、渲染引擎、字体（可搜索的列表）、拼音显示位置。

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSButton, NSPopUpButton};
use qingjian_platform::{CandidateRenderer, Config, LayoutMode, PreeditMode, ThemeMode};

use crate::candidates::typography::{MAX_FONT_SIZE, MIN_FONT_SIZE, Typography};
use crate::candidates::{CandidatePreview, PREVIEW_HEIGHT, available_families};
use crate::preferences::color_picker::ColorPicker;
use crate::preferences::controls::{
    caption, checkbox, note, row_checkbox, row_popup, select, set_checked,
};
use crate::preferences::font_picker::FontPicker;
use crate::preferences::layout::{CONTROL_X, LABEL_WIDTH, Layout, PAGE_PADDING, ROW_HEIGHT};
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

    /// 学习译词字体：搜索框 + 列表。
    font: FontPicker,

    candidate_font: FontPicker,

    candidate_size: Retained<NSPopUpButton>,

    annotation_size: Retained<NSPopUpButton>,

    pos_size: Retained<NSPopUpButton>,

    candidate_bold: Retained<NSButton>,

    annotation_bold: Retained<NSButton>,

    /// 与真实候选窗口共用绘制方式的双向实时示例。
    preview: CandidatePreview,

    colors: [ColorPicker; 6],

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
        let families = available_families(mtm);
        let candidate_font = FontPicker::build(
            layout,
            mtm,
            "中文字体",
            families.clone(),
            Setting::CandidateFont,
        );
        let (candidate_size, candidate_bold) = font_size_row(
            layout,
            mtm,
            "中文字号",
            Setting::CandidateFontSize,
            Setting::CandidateBold,
            target,
        );
        let font = FontPicker::build(layout, mtm, "外语字体", families, Setting::Font);
        let (annotation_size, annotation_bold) = font_size_row(
            layout,
            mtm,
            "外语字号",
            Setting::AnnotationFontSize,
            Setting::AnnotationBold,
            target,
        );
        let sizes: Vec<String> = (MIN_FONT_SIZE..=MAX_FONT_SIZE)
            .map(|size| format!("{size} 磅"))
            .collect();
        let pos_size = row_popup(
            layout,
            mtm,
            "词性字号",
            &sizes,
            Setting::PosFontSize,
            target,
        );
        note(
            layout,
            mtm,
            "中英文互译时，字体、字号和加粗跟随文字语言。词性沿用外语字体，字号单独设置。选中即保存；缺失字形自动使用系统字体。",
        );
        layout.space(8.0);
        let colors = [
            ("整体底色", Setting::CandidateBackgroundColor),
            ("候选字颜色", Setting::CandidateTextColor),
            ("单词词性颜色", Setting::CandidatePosColor),
            ("普通单词颜色", Setting::CandidateWordColor),
            ("不熟单词颜色", Setting::CandidateFreshWordColor),
            ("选中区域颜色", Setting::CandidateHighlightColor),
        ]
        .map(|(title, setting)| ColorPicker::build(layout, mtm, title, setting, target));
        note(
            layout,
            mtm,
            "普通词与不熟单词分别设色，熟悉后自动使用普通词色。恢复默认只还原当前项。预览中 learn 展示不熟单词色，中文释义展示普通词色。",
        );
        let preview = CandidatePreview::new(mtm, layout.control_width());
        let preview_title = caption(mtm, "实时预览");
        layout.place(&preview_title, PAGE_PADDING, LABEL_WIDTH, ROW_HEIGHT);
        layout.place(
            preview.view(),
            CONTROL_X,
            layout.control_width(),
            PREVIEW_HEIGHT,
        );
        layout.next_row(PREVIEW_HEIGHT);
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
            candidate_font,
            candidate_size,
            annotation_size,
            pos_size,
            candidate_bold,
            annotation_bold,
            preview,
            colors,
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
        self.candidate_font.sync(&general.candidate_font);
        let typography = Typography::from(general);
        select(
            &self.candidate_size,
            Some(usize::from(typography.candidate_size - MIN_FONT_SIZE)),
        );
        select(
            &self.annotation_size,
            Some(usize::from(typography.annotation_size - MIN_FONT_SIZE)),
        );
        set_checked(&self.candidate_bold, typography.candidate_bold);
        select(
            &self.pos_size,
            Some(usize::from(typography.pos_size - MIN_FONT_SIZE)),
        );
        set_checked(&self.annotation_bold, typography.annotation_bold);
        self.preview.sync(general);
        let values = [
            &general.candidate_background_color,
            &general.candidate_text_color,
            &general.candidate_pos_color,
            &general.candidate_word_color,
            &general.candidate_fresh_word_color,
            &general.candidate_highlight_color,
        ];
        let swatches = self.preview.color_swatches();
        let appearance = self.preview.appearance();
        for ((picker, value), color) in self.colors.iter().zip(values).zip(&swatches) {
            picker.sync(value, color, &appearance);
        }
        select(
            &self.preedit,
            PreeditMode::ALL.iter().position(|p| *p == general.preedit),
        );
    }
}

/// 同一行放字号与加粗，避免页面为两组设置多占两行。
fn font_size_row(
    layout: &mut Layout,
    mtm: MainThreadMarker,
    title: &str,
    size_setting: Setting,
    bold_setting: Setting,
    target: &PreferencesTarget,
) -> (Retained<NSPopUpButton>, Retained<NSButton>) {
    let bold = checkbox(mtm, "加粗", bold_setting, target);
    layout.place(&bold, CONTROL_X + 216.0, 90.0, ROW_HEIGHT);
    let sizes: Vec<String> = (MIN_FONT_SIZE..=MAX_FONT_SIZE)
        .map(|size| format!("{size} 磅"))
        .collect();
    let size = row_popup(layout, mtm, title, &sizes, size_setting, target);
    (size, bold)
}
