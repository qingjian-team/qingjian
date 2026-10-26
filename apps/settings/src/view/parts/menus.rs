//! 下拉框弹出的菜单：贴在下拉框下面（下面放不下就往上），选中即写配置。

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{Menu, RenderError, Ui};
use qingjian_core::{LEARNING_LANGUAGES, TraditionalVariant};
use qingjian_platform::{EmojiCount, KeyHintMode, Scheme};

use crate::message::{Message, Popup};
use crate::support::Feature;
use crate::view::Page;

/// 形码方案下拉框里「关」那一项。
pub const CODE_OFF: &str = "关";

const CODE_OFF_KEY: &str = "off";

/// 下拉框的本地名，菜单靠它找位置。
pub fn anchor(popup: Popup) -> &'static str {
    match popup {
        Popup::Font => "font",
        Popup::Scheme => "scheme",
        Popup::Code => "code",
        Popup::LearningLanguage => "learning-language",
        Popup::Traditional => "traditional",
        Popup::Emoji => "emoji",
        Popup::Kaomoji => "kaomoji",
        Popup::Symbols => "symbols",
        Popup::KeyHint => "key-hint",
        Popup::AuxScheme => "aux-scheme",
    }
}

/// 下拉框还没画过（拿不到位置）时不出。
/// 拼音下拉的选项（名字，配置写法）：内置的按 [`Scheme::ALL`]，用户双拼方案插在内置双拼后面
/// （输入法还不读方案目录的平台不列）。
pub fn scheme_options(page: &Page) -> Vec<(String, String)> {
    let builtin = |scheme: &Scheme| (scheme.label().to_owned(), scheme.key().to_owned());
    let custom = page
        .settings
        .library
        .shuangpin_schemes
        .iter()
        .filter(|_| Feature::Schemes.here())
        .map(|entry| (entry.label.clone(), entry.id.clone()));
    let split = Scheme::ALL
        .iter()
        .rposition(|scheme| matches!(scheme, Scheme::Shuangpin(_)))
        .map_or(Scheme::ALL.len(), |last| last + 1);
    Scheme::ALL[..split]
        .iter()
        .map(builtin)
        .chain(custom)
        .chain(Scheme::ALL[split..].iter().map(builtin))
        .collect()
}

/// 形码下拉的选项（名字，配置写法）：「关」在最前，后面是各套方案。
pub fn code_options(page: &Page) -> Vec<(String, String)> {
    std::iter::once((CODE_OFF.to_owned(), CODE_OFF_KEY.to_owned()))
        .chain(
            page.settings
                .library
                .code_schemes
                .iter()
                .map(|entry| (entry.label.clone(), entry.id.clone())),
        )
        .collect()
}

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
                let options = scheme_options(page);
                let current = general.scheme.trim();
                let selected = options
                    .iter()
                    .position(|(_, key)| *key == current)
                    .or_else(|| {
                        let current = general.scheme();
                        options.iter().position(|(_, key)| *key == current.key())
                    });
                let (labels, values) = options.into_iter().unzip();
                ("scheme", labels, strings(values), selected)
            }
            Popup::Code => {
                let current = general.code().unwrap_or_else(|| CODE_OFF_KEY.to_owned());
                let (labels, values): (Vec<String>, Vec<String>) =
                    code_options(page).into_iter().unzip();
                let selected = values.iter().position(|value| *value == current);
                ("code", labels, strings(values), selected)
            }
            Popup::LearningLanguage => (
                "learning_language",
                LEARNING_LANGUAGES
                    .iter()
                    .map(|(_, label)| (*label).to_owned())
                    .collect(),
                strings(
                    LEARNING_LANGUAGES
                        .iter()
                        .map(|(code, _)| (*code).to_owned())
                        .collect(),
                ),
                LEARNING_LANGUAGES
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
            Popup::AuxScheme => {
                let aux = &page.settings.config.aux_code;
                let options = page.settings.library.aux_options();
                let ids: Vec<String> = options.iter().map(|entry| entry.stem.clone()).collect();
                let selected = aux
                    .pick(ids.iter().map(String::as_str))
                    .and_then(|id| ids.iter().position(|option| option == id));
                (
                    "scheme",
                    options.iter().map(|entry| entry.name.clone()).collect(),
                    strings(ids),
                    selected,
                )
            }
            Popup::KeyHint => (
                "key_hint",
                KeyHintMode::ALL
                    .iter()
                    .map(|mode| mode.label().to_owned())
                    .collect(),
                strings(
                    KeyHintMode::ALL
                        .iter()
                        .map(|mode| mode.key().to_owned())
                        .collect(),
                ),
                KeyHintMode::ALL
                    .iter()
                    .position(|mode| *mode == general.key_hint),
            ),
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
    let section = match popup {
        Popup::AuxScheme => "aux_code",
        _ => "general",
    };
    // 拼音与形码不能都关（关了就什么也打不了）：一边关着时，另一边的「关」变灰
    let off = match popup {
        Popup::Scheme if general.code().is_none() => Some(Scheme::Off.key()),
        Popup::Code if !general.scheme().is_on() => Some(CODE_OFF_KEY),
        _ => None,
    };
    let disabled: Vec<usize> = off
        .and_then(|off| values.iter().position(|value| value.as_str() == Some(off)))
        .into_iter()
        .collect();
    let items: Vec<&str> = labels.iter().map(String::as_str).collect();
    Menu::new(&items, selected, rect, window)
        .disabled(&disabled)
        .build(
            ui,
            &page.theme,
            "menu",
            move |index| Message::set(section, key, values[index].clone()),
            Message::CloseMenu,
        )
        .map(Some)
}
