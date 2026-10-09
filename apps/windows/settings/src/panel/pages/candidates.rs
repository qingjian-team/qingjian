//! 「候选窗口」页：外观、主题、排布、渲染引擎、字体与字号、过渡动画、拼音显示位置、悬浮状态条。

use qingjian_platform::{Appearance, CandidateRenderer, FontSize, LayoutMode, PreeditMode};
use qingjian_render::ThemeLibrary;
use windows_reactor::*;

use crate::panel::controls::{field, page};
use crate::panel::{Message, Settings};

/// 枚举下拉：按 `label()` 列项，选中 `current`（找不到取 0）。
fn mode_combo<T: PartialEq + Copy>(
    all: &'static [T],
    current: T,
    label: fn(T) -> &'static str,
    callback: Callback<Option<usize>>,
) -> ComboBox {
    ComboBox::new()
        .items_source(all.iter().map(|mode| label(*mode)))
        .selected_index(all.iter().position(|mode| *mode == current).unwrap_or(0))
        .on_selection_changed(callback)
}

/// 字号框：不限范围；没设过显示主题的字号（用户知道从哪个数开始调）。
fn size_box(size: FontSize, theme: Option<f32>, callback: Callback<Option<f64>>) -> NumberBox {
    NumberBox::new()
        .value(size.get().or(theme).map_or(f64::NAN, f64::from))
        .on_value_changed(callback)
}

pub(crate) fn view(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let g = &settings.config.general;
    let font_text = settings
        .font_query
        .clone()
        .unwrap_or_else(|| g.font.clone());
    let query = font_text.to_lowercase();
    let suggestions: Vec<String> = settings
        .families
        .iter()
        .filter(|family| family.to_lowercase().contains(&query))
        .cloned()
        .collect();
    // 内置主题加用户主题目录里的，每次画这一页都重列（新放进去的主题回到这页就能看到）
    let themes = ThemeLibrary::load(qingjian_platform::dirs::themes_dir().as_deref());
    let theme = themes.resolve(g.theme_id(), false);
    let rows = [
        field(
            "外观",
            "",
            mode_combo(
                &Appearance::ALL,
                g.appearance(),
                Appearance::label,
                context.callback(Message::Appearance),
            ),
        ),
        field(
            "主题",
            "只对青简渲染器生效；樱花只有浅色，其余主题按上面的外观切换浅色与深色。",
            ComboBox::new()
                .items_source(themes.themes().iter().map(|theme| theme.name().to_owned()))
                .selected_index(
                    themes
                        .themes()
                        .iter()
                        .position(|theme| theme.id() == g.theme_id())
                        .unwrap_or(0),
                )
                .on_selection_changed(context.callback(Message::Theme)),
        ),
        field(
            "排布",
            "横排时只给高亮的候选显示译词。",
            mode_combo(
                &LayoutMode::ALL,
                g.layout,
                LayoutMode::label,
                context.callback(Message::Layout),
            ),
        ),
        field(
            "渲染引擎",
            "青简渲染器让候选窗口在各平台一致。",
            mode_combo(
                &CandidateRenderer::ALL,
                g.renderer,
                CandidateRenderer::label,
                context.callback(Message::Renderer),
            ),
        ),
        field(
            "字体",
            "只对青简渲染器生效；留空用系统字体，没装的字体自动回到系统字体。",
            AutoSuggestBox::new()
                .width(260.0)
                .text(font_text)
                .placeholder_text("系统字体")
                .items_source(suggestions)
                .on_text_changed(context.callback(Message::FontQuery))
                .on_suggestion_chosen(context.callback(Message::Font)),
        ),
        field(
            "候选字号",
            "单位是点，清空或填 0 回到主题的字号；行高跟着缩放。只对青简渲染器生效。",
            size_box(
                g.candidate_font_size,
                theme.theme_size("candidate"),
                context.callback(Message::CandidateFontSize),
            ),
        ),
        field(
            "译文字号",
            "单位是点，清空或填 0 回到主题的字号。只对青简渲染器生效。",
            size_box(
                g.annotation_font_size,
                theme.theme_size("annotation"),
                context.callback(Message::AnnotationFontSize),
            ),
        ),
        field(
            "",
            "",
            Button::new()
                .on_click(context.message(Message::ResetFontSizes))
                .content("恢复主题字号"),
        ),
        field(
            "过渡动画",
            "高亮换候选时滑过去、主题里的循环动画。关掉后直接跳到位；系统关了「动画效果」时也不播。",
            ToggleSwitch::new()
                .is_on(g.animations)
                .on_toggled(context.callback(Message::Animations)),
        ),
        field(
            "拼音显示",
            "「只在候选窗口」时正在敲的拼音不显示在应用里，终端或行内拼音不正常的应用可以选它。",
            mode_combo(
                &PreeditMode::ALL,
                g.preedit,
                PreeditMode::label,
                context.callback(Message::Preedit),
            ),
        ),
        field(
            "悬浮状态条",
            "桌面上常驻、可拖动的小条：点「中 / 英」切换模式（开着双拼时还显示方案名），点「，。」切全角 / 半角标点，点齿轮打开设置。只在当前输入法是青简时显示，拖到哪下次还在哪。",
            ToggleSwitch::new()
                .is_on(settings.config.status_bar.enabled)
                .on_toggled(context.callback(Message::StatusBar)),
        ),
    ];
    page("候选窗口", StackPanel::new().spacing(16.0).children(rows))
}
