//! 「快捷键」页：翻页键、高亮移动键、模式键，译词 / 翻译选中文字的修饰键。
//! 翻译选中文字只改修饰键，字母键固定用配置里当前的；要换字母直接改 `config.toml`。

use qingjian_platform::{HighlightKeys, Modifiers};
use windows_reactor::{
    Callback, CheckBox, ChildrenControl, ComboBox, ContentControl, Orientation, StackPanel,
    ToggleSwitch, View, ViewContext,
};

use crate::panel::controls::{field, index_of, mode_combo, page};
use crate::panel::{Message, Settings};

/// 翻页键对：界面名 + 配置写法。
pub(crate) const PAGE_KEYS: [(&str, &str); 3] = [
    ("方括号 [ ]", "[]"),
    ("逗号句号 , .", ",."),
    ("减号等号 - =", "-="),
];

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

/// 简繁键预设；任意字母数字组合仍可在配置文件中指定。
pub(crate) const TRADITIONAL_KEYS: [(&str, &str); 4] = [
    ("关闭", ""),
    ("Ctrl + Shift + F", "ctrl+shift+f"),
    ("Ctrl + Alt + F", "ctrl+alt+f"),
    ("Alt + Shift + F", "alt+shift+f"),
];

fn mode_key_combo(current: char, callback: Callback<Option<usize>>) -> ComboBox {
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
            "额外翻页键（可多选）",
            "与下面的主翻页键同时生效。选中的符号在拼音候选中用于翻页；表达式、英文直输段、辅码触发键仍按原有规则处理。",
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(12.0)
                .children(PAGE_KEYS.map(|(label, pair)| {
                    CheckBox::new()
                        .is_checked(s.extra_page_keys.contains(pair))
                        .on_is_checked_changed(
                            context.callback(move |on| Message::ExtraPageKey(pair, on)),
                        )
                        .content(label)
                })),
        ),
        field(
            "全半角标点切换键",
            "打开使用 Ctrl + . 切换当前中英模式的标点。可在配置文件 [shortcut] toggle_punctuation 修改组合键，空串关闭。",
            ToggleSwitch::new()
                .is_on(s.toggle_punctuation.is_some())
                .on_toggled(context.callback(Message::TogglePunctuation)),
        ),
        field(
            "翻页键",
            "选「, .」或「- =」时组句中敲对应符号是翻页，不再是上屏加标点。",
            ComboBox::new()
                .items_source(PAGE_KEYS.iter().map(|(label, _)| *label))
                .selected_index(index_of(&PAGE_KEYS, &settings.config.general.page_keys))
                .on_selection_changed(context.callback(Message::PageKeys)),
        ),
        field(
            "高亮移动键",
            "选「← / →」时 ← / → 在候选之间移动；横排下 ↑ 移到拼音里改光标位置、↓ 回到候选，竖排下拼音光标到末尾按 → 才到下一个候选。",
            mode_combo(
                &HighlightKeys::ALL,
                s.highlight_keys,
                HighlightKeys::label,
                context.callback(Message::HighlightKeys),
            ),
        ),
        field(
            "表达式模式键",
            "",
            mode_key_combo(s.mode.expression, context.callback(Message::ModeExpression)),
        ),
        field(
            "问字模式键",
            "这两个字母开头进模式：v1+2 出 3，usangemu 问「三个木」（需要云服务）。两个键不能相同。",
            mode_key_combo(s.mode.question, context.callback(Message::ModeQuestion)),
        ),
        field(
            "没在输入拼音时敲 ? 也进入问字",
            "开着时 ? 先进问字（中英文模式都行），后面跟字母才是问题，跟其他键时还原成问号；关着问号就是问号。",
            ToggleSwitch::new()
                .is_on(s.mode.question_mark)
                .on_toggled(context.callback(Message::QuestionMark)),
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
            "按需 AI 联想",
            "输入拼音时按这组键 + 当前字母（缺省 Ctrl+Alt+J），获取置顶候选。只改修饰键；字母可在配置的 shortcut.predict 中修改。",
            modifier_combo(
                s.predict.modifiers,
                context.callback(Message::PredictShortcut),
            ),
        ),
        field(
            "切换简繁输出",
            "立即切换候选与上屏文字的简繁字形，保留正在输入的拼音。可关闭以免占用应用快捷键；自定义组合可修改配置文件。",
            ComboBox::new()
                .items_source(
                    TRADITIONAL_KEYS
                        .iter()
                        .map(|(label, _)| (*label).to_owned())
                        .chain(
                            s.toggle_traditional
                                .filter(|key| {
                                    !TRADITIONAL_KEYS
                                        .iter()
                                        .any(|(_, value)| value.parse().ok() == Some(*key))
                                })
                                .map(|key| key.key_string()),
                        ),
                )
                .selected_index(s.toggle_traditional.map_or(0, |key| {
                    TRADITIONAL_KEYS
                        .iter()
                        .position(|(_, value)| value.parse().ok() == Some(key))
                        .unwrap_or(TRADITIONAL_KEYS.len())
                }))
                .on_selection_changed(context.callback(Message::ToggleTraditional)),
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
