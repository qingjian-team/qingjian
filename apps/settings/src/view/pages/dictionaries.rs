//! 「词库」页：随包领域词库（两列勾选），导入的拼音与五笔词库（开关、移除、导入）。基础词库一直启用，不列。

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{RenderError, Ui, group, paragraph, section_title};

use crate::i18n::tr;
use crate::library::Kind;
use crate::message::Message;
use crate::view::Page;
use crate::view::parts::lists::{domain_grid, entry_row, import_row, toggle_disabled};

pub fn dictionaries(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let library = &page.settings.library;
    let config = &page.settings.config.dictionaries;
    let mut out = Vec::new();

    out.push(section_title(
        ui,
        theme,
        tr("dictionaries.section_bundled"),
    )?);
    let domains = if library.domains.is_empty() {
        paragraph(ui, theme, tr("dictionaries.no_bundled"), None)?
    } else {
        domain_grid(ui, page)?
    };
    out.push(group(ui, theme, &[domains])?);

    let mut rows = Vec::new();
    let imported = library
        .dictionaries
        .iter()
        .map(|entry| (Kind::Dictionary, entry))
        .chain(
            library
                .code_words
                .iter()
                .map(|entry| (Kind::CodeWords, entry)),
        );
    for (kind, entry) in imported {
        let toggle = toggle_disabled(kind, &config.disabled, &entry.stem);
        rows.push(entry_row(
            ui,
            page,
            kind,
            entry,
            Some((config.is_enabled(&entry.stem), toggle)),
            true,
            None,
        )?);
    }
    rows.push(import_row(
        ui,
        page,
        Kind::Dictionary,
        tr("dictionaries.import"),
        tr("dictionaries.import_info"),
    )?);
    out.push(section_title(
        ui,
        theme,
        tr("dictionaries.section_imported"),
    )?);
    out.push(group(ui, theme, &rows)?);
    Ok(out)
}
