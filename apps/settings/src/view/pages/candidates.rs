//! 「候选」页：每页几个、译文、输出字形、拼音行声调，候选后面的 emoji / 颜文字 / 符号，排序（学习、本地整句模型），英文词。

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{RenderError, SettingRow, Stepper, Ui, group, section_title};
use qingjian_core::{LEARNING_LANGUAGES, TraditionalVariant};
use qingjian_platform::{EmojiCount, MAX_PAGE_SIZE};

use crate::message::{Message, Popup};
use crate::support::Feature;
use crate::view::Page;
use crate::view::parts::controls::{select, toggle_info, toggle_with};

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
