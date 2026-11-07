//! 「外观」页：预览卡，外观（外观、主题），排布（排布、拼音显示位置），文字（渲染引擎、字体、两个字号、候选字粗细），动效。

mod themes;

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{Button, Checkbox, RenderError, SettingRow, Stepper, Ui, group, section_title};
use qingjian_platform::{Appearance, CandidateRenderer, LayoutMode, PreeditMode};

use crate::i18n::tr;
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
    let appearance = SettingRow::new(tr("appearance.appearance"))
        .control(appearance)
        .build(ui, theme)?;
    let cards = themes::cards(ui, page)?;
    let themes = SettingRow::new(tr("appearance.theme"))
        .control(cards)
        .stacked()
        .build(ui, theme)?;
    out.push(section_title(ui, theme, tr("appearance.appearance"))?);
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
    let mut layout = SettingRow::new(tr("appearance.layout")).control(layout);
    if Feature::HorizontalGrid.here() {
        let grid = Checkbox::new(general.horizontal_grid, tr("appearance.horizontal_grid"))
            .disabled(general.layout != LayoutMode::Horizontal)
            .build(
                ui,
                theme,
                "grid",
                Message::set("general", "horizontal_grid", !general.horizontal_grid),
            )?;
        layout = layout.below(grid);
    }
    if Feature::VerticalReverseAbove.here() {
        let reverse = Checkbox::new(
            general.vertical_reverse_above,
            tr("appearance.reverse_above"),
        )
        .disabled(general.layout != LayoutMode::Vertical)
        .build(
            ui,
            theme,
            "reverse-above",
            Message::set(
                "general",
                "vertical_reverse_above",
                !general.vertical_reverse_above,
            ),
        )?;
        layout = layout.below(reverse);
    }
    if Feature::GridPageKeys.here() {
        let paging = Checkbox::new(general.grid_page_keys, tr("appearance.grid_page_keys"))
            .disabled(general.layout != LayoutMode::Horizontal)
            .build(
                ui,
                theme,
                "grid-page-keys",
                Message::set("general", "grid_page_keys", !general.grid_page_keys),
            )?;
        layout = layout.below(paging);
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
    let preedit = SettingRow::new(tr("appearance.preedit"))
        .info(tr("appearance.preedit_info"))
        .control(preedit)
        .build(ui, theme)?;
    out.push(section_title(ui, theme, tr("appearance.layout"))?);
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
    let renderer = SettingRow::new(tr("appearance.renderer"))
        .control(renderer)
        .build(ui, theme)?;
    // 字体与字号只对青简渲染器生效
    let own = general.renderer == CandidateRenderer::Qingjian;
    let font_name = if general.font.is_empty() {
        tr("common.system_font")
    } else {
        general.font.as_str()
    };
    let font = select(ui, page, Popup::Font, (font_name, &[]), 180.0, !own)?;
    let font = SettingRow::new(tr("appearance.font"))
        .control(font)
        .build(ui, theme)?;
    let candidate = size(ui, page, SizeKind::Candidate, own)?;
    let candidate = SettingRow::new(tr("appearance.candidate_size"))
        .control(candidate)
        .build(ui, theme)?;
    let annotation = size(ui, page, SizeKind::Annotation, own)?;
    let customized = general.candidate_font_size.get().is_some()
        || general.annotation_font_size.get().is_some()
        || general.candidate_font_weight().is_some();
    let reset = Button::new(tr("appearance.reset_sizes"))
        .disabled(!own || !customized)
        .build(ui, theme, "reset-sizes", Message::ResetSizes)?;
    let annotation = SettingRow::new(tr("appearance.annotation_size"))
        .control(annotation)
        .below(reset)
        .build(ui, theme)?;
    let mut rows = vec![renderer, font, candidate];
    if Feature::CandidateFontWeight.here() {
        let value = page.settings.weight();
        let weight = Stepper::new(&value.to_string())
            .limits(value > 100, value < 900)
            .disabled(!own)
            .build(
                ui,
                theme,
                "candidate-weight",
                Message::Weight(-1),
                Message::Weight(1),
            )?;
        rows.push(
            SettingRow::new(tr("appearance.candidate_weight"))
                .info(tr("appearance.candidate_weight_info"))
                .control(weight)
                .build(ui, theme)?,
        );
    }
    rows.push(annotation);
    out.push(section_title(ui, theme, tr("appearance.section_text"))?);
    out.push(group(ui, theme, &rows)?);

    let animations = toggle(
        ui,
        page,
        tr("appearance.animations"),
        ("general", "animations"),
        general.animations,
    )?;
    out.push(section_title(ui, theme, tr("appearance.section_motion"))?);
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
