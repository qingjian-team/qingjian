//! 「生词本」页：顶上切「时间线 / 词表」（记过几门语言时还能切语言），末尾导出与清空。
//! 时间线照「今天遇到了哪些词」排（大数字、记忆色带、按天的一条条），词表按词管理。

mod parts;
mod timeline;
mod words;

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{
    Button, Look, RenderError, Segmented, SettingRow, Ui, group, paragraph, section_title,
};

use crate::i18n::tr;
use crate::message::Message;
use crate::stats::language_name;
use crate::view::Page;
use crate::view::parts::actions::with_notice;
use crate::view::parts::controls::dropdown;
use crate::view::parts::views::view_switch;
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
                let note = paragraph(ui, theme, tr("vocabulary.off"), None)?;
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

/// 顶上一行：视图，记过几门语言时再加语言（竹青是分段控件，另两种外观是下拉框）。
fn switches(ui: &mut Ui<Message>, page: &Page, book: &Vocabulary) -> Result<NodeId, RenderError> {
    let language = if book.languages.len() > 1 {
        let names: Vec<&str> = book.languages.iter().map(|l| language_name(*l)).collect();
        let languages = book.languages.clone();
        let current = languages
            .iter()
            .position(|l| *l == book.language)
            .unwrap_or(0);
        let on_pick = move |index: usize| Message::VocabularyLanguage(languages[index]);
        Some(if page.theme.look == Look::Plain {
            Segmented::new(&names, current).build(
                ui,
                &page.theme,
                "vocabulary-language",
                on_pick,
            )?
        } else {
            dropdown(
                ui,
                page,
                "vocabulary-language",
                (&names, current),
                false,
                on_pick,
            )?
        })
    } else {
        None
    };
    view_switch(
        ui,
        page,
        "vocabulary-view",
        (
            &View::ALL.map(View::label),
            View::ALL.iter().position(|v| *v == book.view).unwrap_or(0),
        ),
        language,
        |index| Message::VocabularyView(View::ALL[index]),
    )
}

/// 导出与清空。
fn manage(
    ui: &mut Ui<Message>,
    page: &Page,
    book: &Vocabulary,
) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let mut out = vec![section_title(ui, theme, tr("vocabulary.section_export"))?];
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
        tr("vocabulary.clear_confirm")
    } else {
        tr("vocabulary.clear")
    });
    if book.confirm_clear {
        clear = clear.primary();
    }
    let clear = clear.build(ui, theme, "vocabulary-clear", Message::VocabularyClear)?;
    let rows = vec![
        with_notice(
            ui,
            page,
            SettingRow::new(tr("vocabulary.export")).control(buttons),
            "vocabulary-export",
        )?,
        with_notice(
            ui,
            page,
            SettingRow::new(tr("vocabulary.clear_book"))
                .info(tr("vocabulary.clear_book_info"))
                .control(clear),
            "vocabulary",
        )?,
    ];
    out.push(group(ui, theme, &rows)?);
    Ok(out)
}
