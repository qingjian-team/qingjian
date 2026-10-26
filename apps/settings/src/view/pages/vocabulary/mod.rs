//! 「生词本」页：顶上切「时间线 / 词表」（记过几门语言时还能切语言），末尾导出与清空。
//! 时间线照「今天遇到了哪些词」排（大数字、记忆色带、按天的一条条），词表按词管理。

mod parts;
mod timeline;
mod words;

use bamboo_slip::taffy::{FlexDirection, FlexWrap, NodeId, Style};
use bamboo_slip::{
    Button, RenderError, Segmented, SettingRow, Ui, Visual, group, paragraph, section_title,
};

use crate::message::Message;
use crate::stats::language_name;
use crate::view::Page;
use crate::view::parts::actions::with_notice;
use crate::vocabulary::{Format, View, Vocabulary};
use parts::row;

pub fn vocabulary(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let empty;
    let book = match &page.settings.vocabulary {
        Some(book) => book,
        None => match Vocabulary::empty(&page.settings.config.general) {
            Some(book) => {
                empty = book;
                &empty
            }
            None => {
                let note = paragraph(
                    ui,
                    theme,
                    "「候选」页的译文选了「不显示」，没有在记生词",
                    None,
                )?;
                return Ok(vec![note]);
            }
        },
    };
    let mut out = vec![switches(ui, page, book)?];
    match book.view {
        View::Timeline => out.extend(timeline::timeline(ui, page, book)?),
        View::Words => out.extend(words::words(ui, page, book)?),
    }
    out.extend(manage(ui, page, book)?);
    Ok(out)
}

/// 顶上一行：视图，记过几门语言时再加语言。
fn switches(ui: &mut Ui<Message>, page: &Page, book: &Vocabulary) -> Result<NodeId, RenderError> {
    let theme = &page.theme;
    let mut children = vec![
        Segmented::new(
            &View::ALL.map(View::label),
            View::ALL.iter().position(|v| *v == book.view).unwrap_or(0),
        )
        .build(ui, theme, "vocabulary-view", |index| {
            Message::VocabularyView(View::ALL[index])
        })?,
    ];
    if book.languages.len() > 1 {
        let names: Vec<&str> = book.languages.iter().map(|l| language_name(*l)).collect();
        let languages = book.languages.clone();
        let current = languages
            .iter()
            .position(|l| *l == book.language)
            .unwrap_or(0);
        children.push(Segmented::new(&names, current).build(
            ui,
            theme,
            "vocabulary-language",
            move |index| Message::VocabularyLanguage(languages[index]),
        )?);
    }
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            gap: page.gap(12.0),
            ..Style::default()
        },
        Visual::Group,
        &children,
    )
}

/// 导出与清空。
fn manage(
    ui: &mut Ui<Message>,
    page: &Page,
    book: &Vocabulary,
) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let mut out = vec![section_title(ui, theme, "导出与清空")?];
    let mut buttons = Vec::new();
    for format in Format::ALL {
        buttons.push(Button::new(format.label()).build(
            ui,
            theme,
            ("vocabulary-export", format.label()),
            Message::VocabularyExport(format),
        )?);
    }
    let buttons = row(ui, page, 8.0, &buttons)?;
    let mut clear = Button::new(if book.confirm_clear {
        "再点一次清空"
    } else {
        "清空"
    });
    if book.confirm_clear {
        clear = clear.primary();
    }
    let clear = clear.build(ui, theme, "vocabulary-clear", Message::VocabularyClear)?;
    let rows = vec![
        with_notice(
            ui,
            page,
            SettingRow::new("导出列出的词")
                .info("导出当前列出的词")
                .control(buttons),
            "vocabulary-export",
        )?,
        with_notice(
            ui,
            page,
            SettingRow::new("清空生词本")
                .info("删掉这门语言的全部记录")
                .control(clear),
            "vocabulary",
        )?,
    ];
    out.push(group(ui, theme, &rows)?);
    Ok(out)
}
