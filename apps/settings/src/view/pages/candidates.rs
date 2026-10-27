//! 「候选」页：每页几个、译文、输出字形、拼音行声调，候选后面的 emoji / 颜文字 / 符号，排序（学习、本地整句模型），英文词。

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{RenderError, SettingRow, Stepper, Ui, group, section_title};
use qingjian_core::{ENGLISH_CANDIDATES_MAX, LEARNING_LANGUAGES, TraditionalVariant};
use qingjian_platform::{EmojiCount, MAX_PAGE_SIZE, NumpadDigit};

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
    let language = LEARNING_LANGUAGES
        .iter()
        .find(|(code, _)| *code == general.learning_language)
        .map_or(general.learning_language.as_str(), |(_, label)| label);
    let names: Vec<&str> = LEARNING_LANGUAGES.iter().map(|(_, name)| *name).collect();
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
        SettingRow::new("每页候选数")
            .control(stepper)
            .build(ui, theme)?,
        SettingRow::new("译文")
            .info("不显示时生词标记也关掉")
            .control(language)
            .build(ui, theme)?,
        SettingRow::new("输出字形")
            .info("软件 → 軟件")
            .control(traditional)
            .build(ui, theme)?,
        toggle_info(
            ui,
            page,
            "译词显示词性",
            Some("如 v. develop"),
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
            SettingRow::new("小键盘数字")
                .info("组句中按小键盘数字")
                .control(numpad)
                .build(ui, theme)?,
        );
    }
    if Feature::ShowTones.here() {
        rows.push(toggle_info(
            ui,
            page,
            "拼音行显示声调",
            Some("如 gān'gà"),
            ("general", "show_tones"),
            general.show_tones,
            false,
        )?);
    }
    out.push(section_title(ui, theme, "显示")?);
    out.push(group(ui, theme, &rows)?);

    let rows = vec![
        count(
            ui,
            page,
            Popup::Emoji,
            "emoji",
            "kaixin → 开心 😄",
            general.emoji,
        )?,
        count(
            ui,
            page,
            Popup::Kaomoji,
            "颜文字",
            "ku → 哭 (T_T)",
            general.kaomoji,
        )?,
        count(
            ui,
            page,
            Popup::Symbols,
            "符号",
            "yuedengyu → 约等于 ≈，alpha → α",
            general.symbols,
        )?,
    ];
    out.push(section_title(ui, theme, "候选后面")?);
    out.push(group(ui, theme, &rows)?);

    let rows = vec![
        toggle_info(
            ui,
            page,
            "学习输入习惯",
            Some("关掉后不再学新习惯"),
            ("general", "learning"),
            general.learning,
            false,
        )?,
        toggle_info(
            ui,
            page,
            "本地整句模型",
            Some("在本机排整句，不联网"),
            ("model", "enabled"),
            config.model.enabled,
            false,
        )?,
    ];
    out.push(section_title(ui, theme, "排序")?);
    out.push(group(ui, theme, &rows)?);

    let rows = vec![
        toggle_info(
            ui,
            page,
            "打拼音时也给英文词",
            Some("hello、compa → company"),
            ("general", "english_in_chinese"),
            general.english_in_chinese,
            false,
        )?,
        SettingRow::new("英文候选最多几个")
            .info("1 时只留最靠前的一条")
            .control(english_max_stepper(
                ui,
                page,
                general.english_max_in_chinese(),
                !general.english_in_chinese,
            )?)
            .build(ui, theme)?,
        SettingRow::new("英文词最高排到第几位")
            .info("有中文候选时；1 为不限")
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
            "中文排在英文词前面",
            ("general", "chinese_first"),
            general.chinese_first,
            !general.english_in_chinese,
        )?,
    ];
    out.push(section_title(ui, theme, "英文词")?);
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
    let control = select(
        ui,
        page,
        popup,
        (
            EmojiCount::LABELS[usize::from(current.get())],
            &EmojiCount::LABELS,
        ),
        120.0,
        false,
    )?;
    SettingRow::new(title)
        .info(example)
        .control(control)
        .build(ui, &page.theme)
}
