//! 「候选」页：每页几个、译文、输出字形、拼音行声调，候选后面的 emoji / 颜文字 / 符号，排序（学习、本地整句模型），英文词。

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{RenderError, SettingRow, Stepper, Ui, group, section_title};
use qingjian_core::{ENGLISH_CANDIDATES_MAX, TraditionalVariant, learning_languages};
use qingjian_platform::{EmojiCount, MAX_PAGE_SIZE, NumpadDigit};

use crate::i18n::tr;
use crate::message::{Message, Popup};
use crate::support::Feature;
use crate::view::Page;
use crate::view::parts::controls::{choice, select, toggle_info, toggle_with};

pub fn candidates(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let config = &page.settings.config;
    let general = &config.general;
    let mut out = Vec::new();

    let size = general.page_size();
    let stepper = Stepper::new(&size.to_string())
        .limits(size > 1, size < MAX_PAGE_SIZE)
        .build(
            ui,
            theme,
            "page-size",
            Message::set("general", "page_size", (size - 1) as i64),
            Message::set("general", "page_size", (size + 1) as i64),
        )?;
    let languages = learning_languages();
    let language = languages
        .iter()
        .find(|(code, _)| *code == general.learning_language)
        .map_or(general.learning_language.as_str(), |(_, label)| label);
    let names: Vec<&str> = languages.iter().map(|(_, name)| *name).collect();
    let language = select(
        ui,
        page,
        Popup::LearningLanguage,
        (language, &names),
        120.0,
        false,
    )?;
    let traditional = select(
        ui,
        page,
        Popup::Traditional,
        (
            general.traditional().label(),
            &TraditionalVariant::ALL.map(TraditionalVariant::label),
        ),
        150.0,
        false,
    )?;
    let mut rows = vec![
        SettingRow::new(tr("candidates.page_size"))
            .control(stepper)
            .build(ui, theme)?,
        SettingRow::new(tr("candidates.translation"))
            .info(tr("candidates.translation_info"))
            .control(language)
            .build(ui, theme)?,
        SettingRow::new(tr("candidates.traditional"))
            .info(tr("candidates.traditional_info"))
            .control(traditional)
            .build(ui, theme)?,
        toggle_info(
            ui,
            page,
            tr("candidates.translation_pos"),
            Some(tr("candidates.translation_pos_info")),
            ("general", "translation_pos"),
            general.translation_pos,
            general.learning_language_off(),
        )?,
    ];
    if Feature::NumpadDigit.here() {
        let numpad = choice(
            ui,
            page,
            "numpad-digit",
            &NumpadDigit::ALL,
            general.numpad_digit,
            NumpadDigit::label,
            |value| Message::set("general", "numpad_digit", value.key()),
        )?;
        rows.insert(
            1,
            SettingRow::new(tr("candidates.numpad_digit"))
                .info(tr("candidates.numpad_digit_info"))
                .control(numpad)
                .build(ui, theme)?,
        );
    }
    if Feature::ShowTones.here() {
        rows.push(toggle_info(
            ui,
            page,
            tr("candidates.show_tones"),
            Some(tr("candidates.show_tones_info")),
            ("general", "show_tones"),
            general.show_tones,
            false,
        )?);
    }
    out.push(section_title(ui, theme, tr("candidates.section_display"))?);
    out.push(group(ui, theme, &rows)?);

    let rows = vec![
        count(
            ui,
            page,
            Popup::Emoji,
            tr("candidates.emoji"),
            tr("candidates.emoji_info"),
            general.emoji,
        )?,
        count(
            ui,
            page,
            Popup::Kaomoji,
            tr("candidates.kaomoji"),
            tr("candidates.kaomoji_info"),
            general.kaomoji,
        )?,
        count(
            ui,
            page,
            Popup::Symbols,
            tr("candidates.symbols"),
            tr("candidates.symbols_info"),
            general.symbols,
        )?,
    ];
    out.push(section_title(ui, theme, tr("candidates.section_extras"))?);
    out.push(group(ui, theme, &rows)?);

    let rows = vec![
        toggle_info(
            ui,
            page,
            tr("candidates.learning"),
            Some(tr("candidates.learning_info")),
            ("general", "learning"),
            general.learning,
            false,
        )?,
        toggle_info(
            ui,
            page,
            tr("candidates.local_model"),
            Some(tr("candidates.local_model_info")),
            ("model", "enabled"),
            config.model.enabled,
            false,
        )?,
    ];
    out.push(section_title(ui, theme, tr("candidates.section_ranking"))?);
    out.push(group(ui, theme, &rows)?);

    let rows = vec![
        toggle_info(
            ui,
            page,
            tr("candidates.english_in_chinese"),
            Some("hello、compa → company"),
            ("general", "english_in_chinese"),
            general.english_in_chinese,
            false,
        )?,
        SettingRow::new(tr("candidates.english_max"))
            .info(tr("candidates.english_max_info"))
            .control(english_max_stepper(
                ui,
                page,
                general.english_max_in_chinese(),
                !general.english_in_chinese,
            )?)
            .build(ui, theme)?,
        SettingRow::new(tr("candidates.english_position"))
            .info(tr("candidates.english_position_info"))
            .control(english_min_stepper(
                ui,
                page,
                general.english_min_position(),
                !general.english_in_chinese,
            )?)
            .build(ui, theme)?,
        toggle_with(
            ui,
            page,
            tr("candidates.chinese_first"),
            ("general", "chinese_first"),
            general.chinese_first,
            !general.english_in_chinese,
        )?,
    ];
    out.push(section_title(ui, theme, tr("candidates.section_english"))?);
    out.push(group(ui, theme, &rows)?);
    Ok(out)
}

/// 中文模式下英文候选最多几个：1 到 [`ENGLISH_CANDIDATES_MAX`]。
fn english_max_stepper(
    ui: &mut Ui<Message>,
    page: &Page,
    current: usize,
    disabled: bool,
) -> Result<NodeId, RenderError> {
    Stepper::new(&current.to_string())
        .limits(current > 1, current < ENGLISH_CANDIDATES_MAX)
        .disabled(disabled)
        .build(
            ui,
            &page.theme,
            "english-max",
            Message::set("general", "english_max_in_chinese", (current - 1) as i64),
            Message::set("general", "english_max_in_chinese", (current + 1) as i64),
        )
}

/// 有中文候选时英文最高排到第几位：1–[`MAX_PAGE_SIZE`]，1 为不限制。
fn english_min_stepper(
    ui: &mut Ui<Message>,
    page: &Page,
    current: usize,
    disabled: bool,
) -> Result<NodeId, RenderError> {
    Stepper::new(&current.to_string())
        .limits(current > 1, current < MAX_PAGE_SIZE)
        .disabled(disabled)
        .build(
            ui,
            &page.theme,
            "english-min-position",
            Message::set("general", "english_min_position", (current - 1) as i64),
            Message::set("general", "english_min_position", (current + 1) as i64),
        )
}

/// 候选后面一次最多配几个：不显示、1–3 个。
fn count(
    ui: &mut Ui<Message>,
    page: &Page,
    popup: Popup,
    title: &str,
    example: &str,
    current: EmojiCount,
) -> Result<NodeId, RenderError> {
    let labels: Vec<String> = (0..=EmojiCount::MAX).map(EmojiCount::label).collect();
    let options: Vec<&str> = labels.iter().map(String::as_str).collect();
    let control = select(
        ui,
        page,
        popup,
        (&EmojiCount::label(current.get()), &options),
        120.0,
        false,
    )?;
    SettingRow::new(title)
        .info(example)
        .control(control)
        .build(ui, &page.theme)
}
