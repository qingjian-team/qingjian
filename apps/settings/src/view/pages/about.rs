//! 「关于」页：版本与链接，更新，许可，数据来源，隐私。

use std::sync::Arc;

use bamboo_slip::taffy::{
    AlignItems, Dimension, FlexDirection, LengthPercentage, NodeId, Rect, Size, Style,
};
use bamboo_slip::{
    BoxPaint, Button, Fill, FontSpec, FontWeight, RenderError, SettingRow, TextStyle, Ui, Visual,
    group, paragraph, section_title,
};
use qingjian_platform::UpdateChannel;

use crate::i18n::{tr, tr_args};
use crate::message::Message;
use crate::system::action::Action;
use crate::view::Page;
use crate::view::parts::actions::{actions, with_notice};
use crate::view::parts::controls::{choice, toggle_info};

/// 随包数据的来源与许可（与旧设置程序「关于」页一致）：（标题, 正文）的文案键。
const SOURCES: [(&str, &str); 9] = [
    ("about.source_dictionary", "about.source_dictionary_text"),
    ("about.source_lm", "about.source_lm_text"),
    ("about.source_glossary", "about.source_glossary_text"),
    ("about.source_emoji", "about.source_emoji_text"),
    ("about.source_english", "about.source_english_text"),
    ("about.source_levels", "about.source_levels_text"),
    ("about.source_wubi", "about.source_wubi_text"),
    ("about.source_chaizi", "about.source_chaizi_text"),
    ("about.source_stroke", "about.source_stroke_text"),
];

pub fn about(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let update = &page.settings.config.update;
    let mut out = vec![header(ui, page)?];

    let channel = choice(
        ui,
        page,
        "update-channel",
        &UpdateChannel::ALL,
        update.channel,
        UpdateChannel::label,
        |value| Message::set("update", "channel", value.key()),
    )?;
    let running = page.settings.update.running();
    let check = Button::new(if running {
        tr("about.checking")
    } else {
        tr("about.check_now")
    })
    .disabled(running)
    .build(ui, theme, "check-update", Message::CheckUpdate)?;
    let mut rows = vec![
        toggle_info(
            ui,
            page,
            tr("about.auto_check"),
            Some(tr("about.auto_check_info")),
            ("update", "check"),
            update.check,
            false,
        )?,
        SettingRow::new(tr("about.channel"))
            .info(tr("about.channel_info"))
            .control(channel)
            .build(ui, theme)?,
        with_notice(
            ui,
            page,
            SettingRow::new(tr("about.check_update")).control(check),
            "update",
        )?,
    ];
    if let Some(found) = page.settings.update.available(update) {
        let title = tr_args("about.new_version", &[("version", &found.version)]);
        let download = Button::new(tr("about.download")).primary().build(
            ui,
            theme,
            "download",
            Message::Run(Action::OpenDownload),
        )?;
        rows.push(SettingRow::new(&title).control(download).build(ui, theme)?);
        let notes = found
            .notes
            .iter()
            .take(6)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        if !notes.is_empty() {
            rows.push(note(ui, page, None, &notes)?);
        }
    }
    out.push(section_title(ui, theme, tr("about.section_update"))?);
    out.push(group(ui, theme, &rows)?);

    out.push(section_title(ui, theme, tr("about.section_license"))?);
    let license = note(ui, page, None, tr("about.license"))?;
    out.push(group(ui, theme, &[license])?);

    let open = page.settings.sources_open;
    let toggle_button = Button::new(if open {
        tr("about.collapse")
    } else {
        tr("about.expand")
    })
    .build(ui, theme, "sources", Message::ToggleSources)?;
    let mut rows = vec![
        SettingRow::new(tr("about.sources"))
            .control(toggle_button)
            .build(ui, theme)?,
    ];
    if open {
        for (title, text) in SOURCES {
            rows.push(note(ui, page, Some(tr(title)), tr(text))?);
        }
    }
    out.push(section_title(ui, theme, tr("about.section_sources"))?);
    out.push(group(ui, theme, &rows)?);

    out.push(section_title(ui, theme, tr("about.section_privacy"))?);
    let privacy = note(ui, page, None, tr("about.privacy"))?;
    out.push(group(ui, theme, &[privacy])?);
    Ok(out)
}

/// logo、「青简」、版本，右边官网与 GitHub。
fn header(ui: &mut Ui<Message>, page: &Page) -> Result<NodeId, RenderError> {
    let theme = &page.theme;
    let mut brand = Vec::new();
    if let Some(logo) = &page.settings.logo {
        let height = page.px(44.0);
        let (width, natural) = logo.size();
        brand.push(ui.scene().node(
            Style {
                size: Size {
                    width: Dimension::length(height * width / natural),
                    height: Dimension::length(height),
                },
                ..Style::default()
            },
            Visual::Box(BoxPaint {
                fill: Some(Fill::Svg {
                    image: Arc::clone(logo),
                    slice: None,
                    px_per_unit: height / natural,
                }),
                border: None,
                radius: 0.0,
            }),
            &[],
        )?);
    }
    let font = FontSpec {
        weight: FontWeight(700),
        family: page.settings.serif,
        ..FontSpec::new(22.0, 30.0)
    };
    let name = ui.scene().node(
        Style::default(),
        Visual::Text {
            text: tr("common.app_name").to_owned(),
            style: TextStyle::new(font.scaled(page.scale), 22.0, theme.colors.text, 1.0),
        },
        &[],
    )?;
    let version = ui.scene().node(
        Style::default(),
        Visual::Text {
            text: tr_args("about.version", &[("version", &env!("CARGO_PKG_VERSION"))]),
            style: theme.text(theme.small, 400, theme.colors.secondary),
        },
        &[],
    )?;
    let titles = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            gap: page.gap(2.0),
            ..Style::default()
        },
        Visual::Group,
        &[name, version],
    )?;
    brand.push(titles);
    let brand = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            gap: page.gap(14.0),
            flex_grow: 1.0,
            ..Style::default()
        },
        Visual::Group,
        &brand,
    )?;
    let links = actions(
        ui,
        page,
        &[
            (tr("about.website"), Action::OpenWebsite),
            ("GitHub", Action::OpenRepository),
        ],
    )?;
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            padding: Rect {
                left: LengthPercentage::length(page.px(4.0)),
                right: LengthPercentage::length(page.px(4.0)),
                top: LengthPercentage::length(page.px(4.0)),
                bottom: LengthPercentage::length(page.px(8.0)),
            },
            ..Style::default()
        },
        Visual::Group,
        &[brand, links],
    )
}

/// 卡片里的一段文字，可带一个小标题。
fn note(
    ui: &mut Ui<Message>,
    page: &Page,
    title: Option<&str>,
    text: &str,
) -> Result<NodeId, RenderError> {
    let theme = &page.theme;
    let mut children = Vec::new();
    if let Some(title) = title {
        children.push(ui.scene().node(
            Style::default(),
            Visual::Text {
                text: title.to_owned(),
                style: theme.text(theme.font.size, 500, theme.colors.text),
            },
            &[],
        )?);
    }
    children.push(paragraph(ui, theme, text, None)?);
    let space = LengthPercentage::length(page.px(12.0));
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            gap: page.gap(4.0),
            padding: Rect {
                top: space,
                bottom: space,
                left: LengthPercentage::length(0.0),
                right: LengthPercentage::length(0.0),
            },
            ..Style::default()
        },
        Visual::Group,
        &children,
    )
}
