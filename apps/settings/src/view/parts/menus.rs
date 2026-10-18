//! 下拉框弹出的菜单：贴在下拉框下面（下面放不下就往上），选中即写配置。

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{Menu, RenderError, Ui};
use qingjian_core::TraditionalVariant;
use qingjian_platform::{EmojiCount, Scheme};

use crate::message::{Message, Popup};
use crate::view::Page;
use crate::view::parts::controls::LANGUAGES;

/// 下拉框的本地名，菜单靠它找位置。
pub fn anchor(popup: Popup) -> &'static str {
    match popup {
        Popup::Font => "font",
        Popup::Scheme => "scheme",
        Popup::LearningLanguage => "learning-language",
        Popup::Traditional => "traditional",
        Popup::Emoji => "emoji",
        Popup::Kaomoji => "kaomoji",
        Popup::Symbols => "symbols",
    }
}

/// 下拉框还没画过（拿不到位置）时不出。
pub fn menu(
    ui: &mut Ui<Message>,
    page: &Page,
    popup: Popup,
    window: (f32, f32),
) -> Result<Option<NodeId>, RenderError> {
    let Some(rect) = ui.rect(ui.key(anchor(popup))) else {
        return Ok(None);
    };
    let general = &page.settings.config.general;
    let strings = |values: Vec<String>| -> Vec<toml_edit::Value> {
        values.into_iter().map(toml_edit::Value::from).collect()
    };
    let (key, labels, values, selected): (&str, Vec<String>, Vec<toml_edit::Value>, Option<usize>) =
        match popup {
            Popup::Font => {
                let families = page.settings.fonts.as_deref().unwrap_or_default();
                let labels = std::iter::once("系统字体".to_owned())
                    .chain(families.iter().cloned())
                    .collect();
                let values: Vec<String> = std::iter::once(String::new())
                    .chain(families.iter().cloned())
                    .collect();
                let selected = values.iter().position(|value| *value == general.font);
                ("font", labels, strings(values), selected)
            }
            Popup::Scheme => {
                let current = general.scheme();
                (
                    "scheme",
                    Scheme::ALL
                        .iter()
                        .map(|scheme| scheme.label().to_owned())
                        .collect(),
                    strings(
                        Scheme::ALL
                            .iter()
                            .map(|scheme| scheme.key().to_owned())
                            .collect(),
                    ),
                    Scheme::ALL.iter().position(|scheme| *scheme == current),
                )
            }
            Popup::LearningLanguage => (
                "learning_language",
                LANGUAGES
                    .iter()
                    .map(|(_, label)| (*label).to_owned())
                    .collect(),
                strings(
                    LANGUAGES
                        .iter()
                        .map(|(code, _)| (*code).to_owned())
                        .collect(),
                ),
                LANGUAGES
                    .iter()
                    .position(|(code, _)| *code == general.learning_language),
            ),
            Popup::Traditional => {
                let current = general.traditional();
                (
                    "traditional_variant",
                    TraditionalVariant::ALL
                        .iter()
                        .map(|variant| variant.label().to_owned())
                        .collect(),
                    strings(
                        TraditionalVariant::ALL
                            .iter()
                            .map(|variant| variant.key().to_owned())
                            .collect(),
                    ),
                    TraditionalVariant::ALL
                        .iter()
                        .position(|variant| *variant == current),
                )
            }
            Popup::Emoji | Popup::Kaomoji | Popup::Symbols => {
                let (key, count) = match popup {
                    Popup::Emoji => ("emoji", general.emoji),
                    Popup::Kaomoji => ("kaomoji", general.kaomoji),
                    _ => ("symbols", general.symbols),
                };
                (
                    key,
                    EmojiCount::LABELS
                        .iter()
                        .map(|&label| label.to_owned())
                        .collect(),
                    (0..=EmojiCount::MAX)
                        .map(|count| toml_edit::Value::from(i64::from(count)))
                        .collect(),
                    Some(usize::from(count.get())),
                )
            }
        };
    let items: Vec<&str> = labels.iter().map(String::as_str).collect();
    Menu::new(&items, selected, rect, window)
        .build(
            ui,
            &page.theme,
            "menu",
            move |index| Message::set("general", key, values[index].clone()),
            Message::CloseMenu,
        )
        .map(Some)
}
