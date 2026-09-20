//! 「通用」页：学习语言、每页候选数、输入方案、英文模式候选。

use qingjian_platform::{MAX_PAGE_SIZE, Scheme, ShiftLetter, SwitchKey};
use windows_reactor::*;

use crate::panel::controls::{feedback, field, index_of, page};
use crate::panel::{Message, Settings};

/// 学习语言：界面名 + 配置写法。
pub(crate) const LANGUAGES: [(&str, &str); 4] = [
    ("英语", "en"),
    ("日语", "ja"),
    ("西班牙语", "es"),
    ("不显示译文", "off"),
];

/// 输入方案：界面名 + 配置写法，直接照 [`Scheme::ALL`] 建，不另抄一份。
/// 数组长度取自 `ALL`，以后加方案时这里数组对不上就编不过。
pub(crate) const SCHEMES: [(&str, &str); Scheme::ALL.len()] = [
    (Scheme::ALL[0].label(), Scheme::ALL[0].key()),
    (Scheme::ALL[1].label(), Scheme::ALL[1].key()),
    (Scheme::ALL[2].label(), Scheme::ALL[2].key()),
    (Scheme::ALL[3].label(), Scheme::ALL[3].key()),
    (Scheme::ALL[4].label(), Scheme::ALL[4].key()),
    (Scheme::ALL[5].label(), Scheme::ALL[5].key()),
    (Scheme::ALL[6].label(), Scheme::ALL[6].key()),
    (Scheme::ALL[7].label(), Scheme::ALL[7].key()),
    (Scheme::ALL[8].label(), Scheme::ALL[8].key()),
];

/// 中英切换键：界面名 + 配置写法，与 [`SwitchKey::ALL`] 同序（有测试钉住）。
pub(crate) const SWITCH_KEYS: [(&str, &str); 4] = [
    (SwitchKey::Shift.label(), SwitchKey::Shift.key()),
    (SwitchKey::Control.label(), SwitchKey::Control.key()),
    (SwitchKey::CtrlSpace.label(), SwitchKey::CtrlSpace.key()),
    (SwitchKey::None.label(), SwitchKey::None.key()),
];

pub(crate) fn string_combo(
    options: &'static [(&str, &str)],
    current: &str,
    callback: Callback<Option<usize>>,
) -> ComboBox {
    ComboBox::new()
        .items_source(options.iter().map(|(label, _)| *label))
        .selected_index(index_of(options, current))
        .on_selection_changed(callback)
}

/// Shift+字母的下拉：选项直接由 [`ShiftLetter::ALL`] 生成，免得再抄一份表（顺序要和它一致）。
fn shift_letter_combo(current: ShiftLetter, callback: Callback<Option<usize>>) -> ComboBox {
    ComboBox::new()
        .items_source(ShiftLetter::ALL.iter().map(|mode| mode.label()))
        .selected_index(ShiftLetter::ALL.iter().position(|mode| *mode == current))
        .on_selection_changed(callback)
}

pub(crate) fn view(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let g = &settings.config.general;
    let english_off = !settings.config.apps.english_candidates_off.is_empty();
    let rows = [
        field(
            "学习语言",
            "候选词右侧显示哪种语言的译词，只列出装了释义表的语言；「不显示译文」同时关掉生词标记与释义兜底。",
            string_combo(
                &LANGUAGES,
                &g.learning_language,
                context.callback(Message::LearningLanguage),
            ),
        ),
        field(
            "每页候选数",
            "",
            NumberBox::new()
                .minimum(1.0)
                .maximum(MAX_PAGE_SIZE as f64)
                .value(g.page_size as f64)
                .on_value_changed(context.callback(Message::PageSize)),
        ),
        field(
            "拼音方案",
            "全拼、五套双拼、大千注音，或关（只用下面的五笔）。\
             双拼下 v、u、i 是音节键，表达式与问字模式改用 Shift+V、Shift+U 进（微软、搜狗方案的 ; 键是 ing）；\
             注音下 v、u、i 也是按键，只能用 ? 开头进。",
            string_combo(
                &SCHEMES,
                g.scheme().key(),
                context.callback(Message::Scheme),
            ),
        ),
        field(
            "五笔（86 版）",
            "与拼音方案同时开着就是混输：编码打全的五笔词在前，打不出的字直接打拼音。\
             单用五笔请把拼音方案关掉；第 5 个字母起五笔查不到东西，自动只剩拼音。\
             译词、生词记录与学习照常。",
            ToggleSwitch::new()
                .is_on(g.wubi())
                .on_toggled(context.callback(Message::Wubi)),
        ),
        field(
            "繁体输出",
            "打字时将候选词转换为繁体中文。",
            ToggleSwitch::new()
                .is_on(g.traditional)
                .on_toggled(context.callback(Message::Traditional)),
        ),
        field(
            "中文模式标点转全角",
            "没在打拼音时敲 , . ? ! 等出「，。？！」，数字后面的点保持半角；悬浮状态条的「，。」格也能切，切的是当前模式那份。",
            ToggleSwitch::new()
                .is_on(g.full_width_punctuation)
                .on_toggled(context.callback(Message::FullWidthPunctuation)),
        ),
        field(
            "英文模式标点转全角",
            "中英各记一份，缺省英文半角。",
            ToggleSwitch::new()
                .is_on(g.english_full_width_punctuation)
                .on_toggled(context.callback(Message::EnglishFullWidthPunctuation)),
        ),
        field(
            "英文模式（Caps Lock）也给候选",
            "Tab 或方向键选词；空格、回车、标点仍原样上屏敲的字母，不选词时与直接打字一样。",
            ToggleSwitch::new()
                .is_on(g.english_candidates)
                .on_toggled(context.callback(Message::EnglishCandidates)),
        ),
        field(
            "但在终端和代码编辑器里不给",
            "终端、Windows Terminal、VS Code、Cursor、JetBrains 等，那里的候选窗口会挡住应用自己的补全；名单可在配置文件里改。",
            ToggleSwitch::new()
                .is_on(english_off)
                .is_enabled(g.english_candidates)
                .on_toggled(context.callback(Message::EnglishOffInApps)),
        ),
        field(
            "输入拼音时中文候选排在英文词前面",
            "开着时整段输入是英文词时（hello、key）英文词排第二，空格上屏的仍是中文；关着（缺省）拼音不成立的输入英文词排第一。",
            ToggleSwitch::new()
                .is_on(g.chinese_first)
                .on_toggled(context.callback(Message::ChineseFirst)),
        ),
        feedback(&settings.notice),
        field(
            "中文模式下的 Shift + 字母",
            "「交给应用」是临时打英文（与以前一致）：拼音先上屏，这个键归应用；\
             「进组句」把它收进拼音缓冲区，匹配时按小写算，所以 Cpan 与 cpan 一样能出「C盘」。",
            shift_letter_combo(g.shift_letter, context.callback(Message::ShiftLetter)),
        ),
        field(
            "中英切换键",
            "单击选中的键（或按 Ctrl + Space）在中英之间切换，改完立刻生效。打字时容易误触 Shift 的话改成「单击 Ctrl」；「不切换」时只剩任务栏 / 悬浮状态条上的「中」「英」按钮。注意 Ctrl + Space 常被编辑器用作代码补全等快捷键，选了它会把这些应用里的该组合键抢过来。",
            string_combo(
                &SWITCH_KEYS,
                settings.config.shortcut.switch_mode.key(),
                context.callback(Message::SwitchMode),
            ),
        ),
        field(
            "启用内置英文模式",
            "关掉后青简固定中文模式：切换键与任务栏、悬浮状态条上的「中」「英」按钮都不再切到英文，需要英文时用系统快捷键（Win + Space）切到别的输入法。",
            ToggleSwitch::new()
                .is_on(g.english_mode)
                .on_toggled(context.callback(Message::EnglishMode)),
        ),
    ];
    page("通用", StackPanel::new().spacing(16.0).children(rows))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 下拉的项与配置枚举一一对应，顺序也一样（下标就是 `SwitchKey::ALL` 的下标）。
    #[test]
    fn switch_key_options_follow_the_config_enum() {
        assert_eq!(SWITCH_KEYS.len(), SwitchKey::ALL.len());
        for (index, key) in SwitchKey::ALL.into_iter().enumerate() {
            assert_eq!(SWITCH_KEYS[index], (key.label(), key.key()));
        }
    }
}
