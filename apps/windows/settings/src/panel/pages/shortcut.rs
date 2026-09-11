//! 「快捷键」页：翻页键、模式键，译词 / 删候选 / 翻译选中文字的修饰键。
//! 翻译选中文字只改修饰键，字母键固定用配置里当前的；要换字母直接改 `config.toml`。

use qingjian_platform::Modifiers;
use windows_reactor::*;

use crate::panel::controls::{field, index_of, page};
use crate::panel::{Message, Settings};

/// 翻页键对：界面名 + 配置写法。
pub(crate) const PAGE_KEYS: [(&str, &str); 2] = [("方括号 [ ]", "[]"), ("逗号句号 , .", ",.")];

/// 可当模式键的字母（与 Core `ModeKeys::CANDIDATES` 一致）。
pub(crate) const MODE_KEYS: [&str; 3] = ["v", "u", "i"];

/// 修饰键预设：界面名 + 配置写法。
pub(crate) const MODIFIERS: [(&str, &str); 6] = [
    ("Ctrl", "ctrl"),
    ("Alt", "alt"),
    ("Shift", "shift"),
    ("Ctrl + Shift", "shift+ctrl"),
    ("Ctrl + Alt", "ctrl+alt"),
    ("Alt + Shift", "shift+alt"),
];

fn mode_combo(current: char, callback: Callback<Option<usize>>) -> ComboBox {
    let selected = MODE_KEYS
        .iter()
        .position(|key| key.starts_with(current))
        .unwrap_or(0);
    ComboBox::new()
        .items_source(MODE_KEYS)
        .selected_index(selected)
        .on_selection_changed(callback)
}

/// 按解析后相等找当前项，不依赖字符串写法。
fn modifier_combo(current: Modifiers, callback: Callback<Option<usize>>) -> ComboBox {
    let selected = MODIFIERS
        .iter()
        .position(|(_, value)| value.parse::<Modifiers>().ok() == Some(current))
        .unwrap_or(0);
    ComboBox::new()
        .items_source(MODIFIERS.iter().map(|(label, _)| *label))
        .selected_index(selected)
        .on_selection_changed(callback)
}

pub(crate) fn view(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let s = &settings.config.shortcut;
    let rows = [
        field(
            "翻页键",
            "选「, .」时组句中敲逗号句号是翻页，不再是上屏加标点。",
            ComboBox::new()
                .items_source(PAGE_KEYS.iter().map(|(label, _)| *label))
                .selected_index(index_of(&PAGE_KEYS, &settings.config.general.page_keys))
                .on_selection_changed(context.callback(Message::PageKeys)),
        ),
        field(
            "表达式模式键",
            "",
            mode_combo(s.mode.expression, context.callback(Message::ModeExpression)),
        ),
        field(
            "问字模式键",
            "这两个字母开头进模式：v1+2 出 3，usangemu 问「三个木」（需要云服务）；? 开头永远是问字。两个键不能相同。",
            mode_combo(s.mode.question, context.callback(Message::ModeQuestion)),
        ),
        field(
            "译词上屏（第一个）",
            "按住修饰键再按候选序号，上屏候选右侧的译词而不是中文。",
            modifier_combo(s.translation, context.callback(Message::Translation)),
        ),
        field(
            "译词上屏（第二个）",
            "候选有两个译词时，这组键上屏后一个。两组不能相同。Ctrl + Shift 是 Windows 切换输入法的热键，同时装着别的输入法时别选它。",
            modifier_combo(
                s.translation_second,
                context.callback(Message::TranslationSecond),
            ),
        ),
        field(
            "删除候选",
            "按住修饰键再按候选序号：自己造的词、云端选过的词整删；词库里的词清掉学习记录，回到原排序。",
            modifier_combo(
                s.delete_candidate,
                context.callback(Message::DeleteCandidate),
            ),
        ),
        field(
            "翻译选中文字",
            "选中一段文字后按这组键 + 当前字母（缺省 Ctrl+Alt+T），把它译成学习语言，回车 / 空格替换、Esc 保留原文。需要云服务。这里只改修饰键，字母固定用当前的。",
            modifier_combo(
                s.translate_selection.modifiers,
                context.callback(Message::TranslateSelection),
            ),
        ),
    ];
    page("快捷键", StackPanel::new().spacing(16.0).children(rows))
}
