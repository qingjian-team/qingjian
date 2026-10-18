//! 「外观」页：预览卡，外观（外观、主题），排布（排布、拼音显示位置），文字（渲染引擎、字体、两个字号），动效。

mod themes;

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{Button, Checkbox, RenderError, SettingRow, Stepper, Ui, group, section_title};
use qingjian_platform::{Appearance, CandidateRenderer, LayoutMode, PreeditMode};

use crate::message::{Message, Popup, SizeKind};
use crate::support::Feature;
use crate::view::Page;
use crate::view::parts::controls::{choice, select, toggle};

pub fn appearance(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let general = &page.settings.config.general;
    let mut out = vec![themes::preview(ui, page)?];

    let appearance = choice(
        ui,
        page,
        "appearance",
        &Appearance::ALL,
        general.appearance(),
        Appearance::label,
        |value| Message::set("general", "appearance", value.key()),
    )?;
    let appearance = SettingRow::new("外观")
        .control(appearance)
        .build(ui, theme)?;
    let cards = themes::cards(ui, page)?;
    let themes = SettingRow::new("主题")
        .control(cards)
        .stacked()
        .build(ui, theme)?;
    out.push(section_title(ui, theme, "外观")?);
    out.push(group(ui, theme, &[appearance, themes])?);

    let layout = choice(
        ui,
        page,
        "layout",
        &LayoutMode::ALL,
        general.layout,
        LayoutMode::label,
        |value| Message::set("general", "layout", value.key()),
    )?;
    let mut layout = SettingRow::new("排布").control(layout);
    if Feature::HorizontalGrid.here() {
        let grid = Checkbox::new(general.horizontal_grid, "横排时 ↑ / ↓ 展开成多行")
            .disabled(general.layout != LayoutMode::Horizontal)
            .build(
                ui,
                theme,
                "grid",
                Message::set("general", "horizontal_grid", !general.horizontal_grid),
            )?;
        layout = layout.below(grid);
    }
    let layout = layout.build(ui, theme)?;
    let preedit = choice(
        ui,
        page,
        "preedit",
        &PreeditMode::ALL,
        general.preedit,
        PreeditMode::label,
        |value| Message::set("general", "preedit", value.key()),
    )?;
    let preedit = SettingRow::new("拼音显示位置")
        .info("终端里行内拼音显示不正常时选「只在窗口」")
        .control(preedit)
        .build(ui, theme)?;
    out.push(section_title(ui, theme, "排布")?);
    out.push(group(ui, theme, &[layout, preedit])?);

    let renderer = choice(
        ui,
        page,
        "renderer",
        &CandidateRenderer::ALL,
        general.renderer,
        CandidateRenderer::label,
        |value| Message::set("general", "renderer", value.key()),
    )?;
    let renderer = SettingRow::new("渲染引擎")
        .control(renderer)
        .build(ui, theme)?;
    // 字体与字号只对青简渲染器生效
    let own = general.renderer == CandidateRenderer::Qingjian;
    let font_name = if general.font.is_empty() {
        "系统字体"
    } else {
        general.font.as_str()
    };
    let font = select(ui, page, Popup::Font, (font_name, &[]), 180.0, !own)?;
    let font = SettingRow::new("字体").control(font).build(ui, theme)?;
    let candidate = size(ui, page, SizeKind::Candidate, own)?;
    let candidate = SettingRow::new("候选字号")
        .control(candidate)
        .build(ui, theme)?;
    let annotation = size(ui, page, SizeKind::Annotation, own)?;
    let customized =
        general.candidate_font_size.get().is_some() || general.annotation_font_size.get().is_some();
    let reset = Button::new("恢复主题字号")
        .disabled(!own || !customized)
        .build(ui, theme, "reset-sizes", Message::ResetSizes)?;
    let annotation = SettingRow::new("译文字号")
        .control(annotation)
        .below(reset)
        .build(ui, theme)?;
    out.push(section_title(ui, theme, "文字")?);
    out.push(group(ui, theme, &[renderer, font, candidate, annotation])?);

    let animations = toggle(
        ui,
        page,
        "过渡动画",
        ("general", "animations"),
        general.animations,
    )?;
    out.push(section_title(ui, theme, "动效")?);
    out.push(group(ui, theme, &[animations])?);
    Ok(out)
}

fn size(
    ui: &mut Ui<Message>,
    page: &Page,
    kind: SizeKind,
    enabled: bool,
) -> Result<NodeId, RenderError> {
    let value = page.settings.size(kind);
    let text = format!("{} pt", trim(value));
    let local = match kind {
        SizeKind::Candidate => "candidate-size",
        SizeKind::Annotation => "annotation-size",
    };
    Stepper::new(&text)
        .limits(value > 6.0, value < 72.0)
        .disabled(!enabled)
        .build(
            ui,
            &page.theme,
            local,
            Message::Size(kind, -1),
            Message::Size(kind, 1),
        )
}

/// 整数不带小数点，其余留一位。
fn trim(value: f32) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}
