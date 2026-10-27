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

use crate::message::Message;
use crate::system::action::Action;
use crate::view::Page;
use crate::view::parts::actions::{actions, with_notice};
use crate::view::parts::controls::{choice, toggle_info};

const LICENSE: &str = "自由软件，GPL-3.0-or-later 许可证：可以自由使用、修改与再分发，修改后分发须同样开源。官方渠道免费。";

/// 随包数据的来源与许可（与旧设置程序「关于」页一致）。
const SOURCES: [(&str, &str); 9] = [
    (
        "词库",
        "通用规范汉字表；现代汉语常用词表（liuxilu 校对版）；THUOCL（清华大学自然语言处理实验室，MIT）；读音取自 Unihan（Unicode License v3）。",
    ),
    (
        "语言模型",
        "中文维基百科（CC BY-SA 4.0）与 LCCC（清华大学 CoAI，MIT）语料统计。",
    ),
    ("释义表", "由大语言模型（DeepSeek）生成，青简自建。"),
    ("emoji", "Unicode CLDR annotations（Unicode License v3）。"),
    (
        "英文词表",
        "ESDB / SCOWL（© Kevin Atkinson，按其许可保留版权声明）；CSpell 词典（MIT）。",
    ),
    (
        "词汇等级",
        "The CEFR-J Wordlist Version 1.5（Yukio Tono，Tokyo University of Foreign Studies，cefr-j.org）；Octanove Vocabulary Profile C1/C2（CC BY-SA 4.0）；JLPT 词表（tanos.co.uk，CC BY；经 elzup/jlpt-word-list 整理，MIT）。",
    ),
    (
        "五笔码表",
        "86 五笔极点码表（sxjudya/rime-wubi86-jidian，Apache-2.0）；编码来自上游，词频由青简词库按词面回填。",
    ),
    (
        "拆字表",
        "開放詞典網漢語拆字字典（kaifangcidian.com，CC BY 3.0）；按部件读音重新编排，读音取自青简词库。",
    ),
    (
        "笔画码表",
        "CNS11643 全字庫筆順資料（數位發展部，政府資料開放授權條款第 1 版 / OFL-1.1）；大陆笔顺按通用规范字表校正。",
    ),
];

const PRIVACY: &str = "青简不上传任何数据。开着「自动检查更新」时每天向官网读一次版本列表，请求不带任何标识。开着云联想或翻译时，光标附近的文字与拼音会发给你在「云服务」页填的 AI 服务商的服务器，不经过作者。输入日志只写在这台电脑上，可以在「高级」页关掉或清空。";

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
        "正在检查…"
    } else {
        "立即检查"
    })
    .disabled(running)
    .build(ui, theme, "check-update", Message::CheckUpdate)?;
    let mut rows = vec![
        toggle_info(
            ui,
            page,
            "自动检查更新",
            Some("有新版时提示，不自动安装"),
            ("update", "check"),
            update.check,
            false,
        )?,
        SettingRow::new("渠道")
            .info("测试版会更早收到新功能")
            .control(channel)
            .build(ui, theme)?,
        with_notice(
            ui,
            page,
            SettingRow::new("检查更新").control(check),
            "update",
        )?,
    ];
    if let Some(found) = page.settings.update.available(update) {
        let title = format!("新版本 {}", found.version);
        let download = Button::new("下载").primary().build(
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
    out.push(section_title(ui, theme, "更新")?);
    out.push(group(ui, theme, &rows)?);

    out.push(section_title(ui, theme, "许可")?);
    let license = note(ui, page, None, LICENSE)?;
    out.push(group(ui, theme, &[license])?);

    let open = page.settings.sources_open;
    let toggle_button = Button::new(if open { "收起" } else { "展开" }).build(
        ui,
        theme,
        "sources",
        Message::ToggleSources,
    )?;
    let mut rows = vec![
        SettingRow::new("词库、模型、词表与码表")
            .control(toggle_button)
            .build(ui, theme)?,
    ];
    if open {
        for (title, text) in SOURCES {
            rows.push(note(ui, page, Some(title), text)?);
        }
    }
    out.push(section_title(ui, theme, "数据来源")?);
    out.push(group(ui, theme, &rows)?);

    out.push(section_title(ui, theme, "隐私")?);
    let privacy = note(ui, page, None, PRIVACY)?;
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
            text: "青简".to_owned(),
            style: TextStyle::new(font.scaled(page.scale), 22.0, theme.colors.text, 1.0),
        },
        &[],
    )?;
    let version = ui.scene().node(
        Style::default(),
        Visual::Text {
            text: format!("版本 {}", env!("CARGO_PKG_VERSION")),
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
            ("官网", Action::OpenWebsite),
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
