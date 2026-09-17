//! 键路由测试：组句、选词、标点、翻页、模式与快捷键。走 assets/sample 真数据。

use super::*;

#[test]
fn nihao_out_candidates_and_space_commits() {
    let mut h = sample_host();
    type_str(&mut h, "nihao");
    assert!(h.composing());
    assert!(!h.layout.is_empty(), "应有候选");
    assert!(!h.preedit.is_empty(), "preedit 应显示拼音");
    assert!(h.key(0x20, 0, false), "空格应被吞掉");
    let committed = h.pending_commit.take().expect("应有上屏文本");
    assert!(!committed.is_empty());
    assert!(!h.composing(), "nihao 应整体上屏、缓冲清空");
}

#[test]
fn digit_selects_on_page() {
    let mut h = sample_host();
    type_str(&mut h, "ni");
    let second = h.layout.candidate(1).map(|c| c.text.clone());
    assert!(h.key(0x32, 0, false), "数字 2 应被吞掉");
    if let Some(expected) = second {
        assert_eq!(h.pending_commit.take().unwrap(), expected);
    }
}

#[test]
fn backspace_then_escape_clears() {
    let mut h = sample_host();
    type_str(&mut h, "ni");
    assert!(h.key(0xff08, 0, false));
    assert!(h.composing(), "退一格还剩 n");
    assert!(h.key(0xff1b, 0, false));
    assert!(!h.composing(), "Esc 应清空");
    assert!(h.pending_commit.is_none());
}

#[test]
fn passthrough_when_idle() {
    let mut h = sample_host();
    assert!(!h.key(0x20, 0, false), "空闲时空格透传");
    assert!(!h.key(0x31, 0, false), "空闲时数字透传");
    assert!(!h.key(0xff08, 0, false), "空闲时退格透传");
    assert!(!h.key(0x61, 1 << 2, false), "Ctrl+a 透传");
}

#[test]
fn candidates_carry_translation() {
    let mut h = sample_host();
    type_str(&mut h, "ni");
    let translated = (0..h.layout.len())
        .filter_map(|i| h.layout.candidate(i))
        .any(|c| c.translation.is_some());
    assert!(translated, "样例释义表下应至少有一个候选带译文");
}

#[test]
fn enter_commits_raw_pinyin() {
    let mut h = sample_host();
    type_str(&mut h, "nihao");
    assert!(h.key(0xff0d, 0, false));
    assert_eq!(h.pending_commit.take().unwrap(), "nihao");
    assert!(!h.composing());
}

#[test]
fn punctuation_idle_full_width() {
    let mut h = sample_host();
    assert!(h.key(0x2c, 0, false), "逗号应转全角并吞掉");
    assert_eq!(h.pending_commit.take().unwrap(), "\u{ff0c}");
}

#[test]
fn punctuation_while_composing_enters_raw_segment() {
    // 组句中敲半角标点（翻页键除外）：进缓冲区成英文直输段(hello, dui'ma?),
    // 与 macOS 实现、docs/user/input/shortcuts.md 口径一致。
    let mut h = sample_host();
    type_str(&mut h, "ni");
    assert!(h.key(0x2c, 0, false), ", 应进缓冲区");
    assert!(h.composing());
    assert!(h.engine.raw_mode(), "标点应把整段变成直输段");
    assert_eq!(h.engine.composition().text(), "ni,");
    assert!(h.pending_commit.is_none(), "不应上屏候选或全角标点");
    assert!(h.key(0xff0d, 0, false));
    assert_eq!(h.pending_commit.take().unwrap(), "ni,");
}

#[test]
fn apostrophe_separates_syllables() {
    // 音节分隔符 ':xi'an 组句中进缓冲区，不当标点。
    let mut h = sample_host();
    type_str(&mut h, "xi");
    assert!(h.key(0x27, 0, false), "' 应进缓冲区");
    assert!(h.composing());
    assert!(h.pending_commit.is_none());
    type_str(&mut h, "an");
    assert_eq!(h.engine.composition().text(), "xi'an");
}

#[test]
fn shuangpin_semicolon_completes_syllable() {
    // 微软/搜狗双拼的 ； 是 ing 键：末尾落单声母时进缓冲区，不当标点。
    let mut config = qingjian_platform::Config::default();
    config.general.shuangpin = "microsoft".to_owned();
    let mut h = host_with(config);
    type_str(&mut h, "x");
    assert!(h.engine.takes_semicolon(), "落单声母 x 后 ; 应是 ing");
    assert!(h.key(0x3b, 0, false), "; 应进缓冲区");
    assert_eq!(h.engine.composition().text(), "x;");
    assert!(h.pending_commit.is_none());
}

#[test]
fn english_composing_takes_digits_uppercase_apostrophe() {
    // 英文组句中数字、大写、' 进缓冲区(win32 / McDonald / don't)，不选词不打断。
    let mut h = sample_host();
    h.key(0xffe1, 0, false);
    h.key(0xffe1, 1, true);
    assert!(h.engine.english_mode());
    type_str(&mut h, "win");
    assert!(h.key(0x33, 0, false), "英文组句中 3 应进缓冲区");
    assert_eq!(h.engine.composition().text(), "win3");
    h.key(0xff1b, 0, false);
    type_str(&mut h, "don");
    assert!(h.key(0x27, 0, false), "英文组句中 ' 应进缓冲区");
    assert_eq!(h.engine.composition().text(), "don'");
    h.key(0xff1b, 0, false);
    type_str(&mut h, "mc");
    assert!(h.key(0x44, 1, false), "英文组句中大写 D 应进缓冲区");
    assert_eq!(h.engine.composition().text(), "mcD");
}

#[test]
fn page_keys_dash_equals_config_beats_raw_entry() {
    // 用户显式配回 page_keys = "-="：翻页优先于 - 的直输段入口。
    let mut config = qingjian_platform::Config::default();
    config.general.page_keys = "-=".to_owned();
    let mut h = host_with(config);
    type_str(&mut h, "s");
    assert!(h.page_count() > 1);
    assert!(h.key(0x3d, 0, false), "= 应翻下一页");
    assert_eq!(h.page, 1);
    assert!(h.key(0x2d, 0, false), "- 应翻回上一页");
    assert_eq!(h.page, 0);
    assert!(!h.engine.raw_mode(), "配置为翻页键的 - 不应进直输段");
}

#[test]
fn shift_tap_toggles_english_mode() {
    let mut h = sample_host();
    assert!(!h.engine.english_mode());
    assert!(!h.key(0xffe1, 0, false), "Shift 按下透传");
    assert!(!h.key(0xffe1, 1, true), "Shift 松开透传");
    assert!(h.engine.english_mode(), "轻点应切到英文模式");
    // 夹了别的键就不算轻点
    assert!(!h.key(0xffe1, 0, false));
    h.key(0x61, 1, false);
    assert!(!h.key(0xffe1, 1, true));
    assert!(h.engine.english_mode(), "夹键后松开不应再切换");
}

#[test]
fn shifted_letter_flushes_raw_and_passes() {
    let mut h = sample_host();
    type_str(&mut h, "ni");
    assert!(!h.key(0x4e, 1, false), "大写 N 应透传");
    assert_eq!(h.pending_commit.take().unwrap(), "ni", "拼音应原样上屏");
    assert!(!h.composing());
}

#[test]
fn private_mode_smoke() {
    let mut h = sample_host();
    h.set_private(true);
    type_str(&mut h, "nihao");
    assert!(h.key(0x20, 0, false));
    assert!(
        h.pending_commit.take().is_some(),
        "私密模式照常上屏,只是不学习"
    );
    h.set_private(false);
}

#[test]
fn keypad_digit_selects() {
    let mut h = sample_host();
    type_str(&mut h, "ni");
    let second = h.layout.candidate(1).map(|c| c.text.clone());
    assert!(h.key(0xffb2, 0, false), "小键盘 2 应被吞掉");
    if let Some(expected) = second {
        assert_eq!(h.pending_commit.take().unwrap(), expected);
    }
}

#[test]
fn translation_shortcut_commits_gloss() {
    let mut h = sample_host();
    // 找到当前页第一个带译文的候选，用 Alt+对应数字上屏其译文。
    let mut target = None;
    type_str(&mut h, "ni");
    for off in 0..h.layout.page_size() {
        if let Some(c) = h.layout.candidate(off)
            && let Some(t) = &c.translation
            && let Some(sense) = t.senses().first()
        {
            target = Some((off, sense.text.clone()));
            break;
        }
    }
    let (off, gloss) = target.expect("样例里应有带译文的候选");
    // Alt = 1<<3 = 0x8；数字键 '1'+off。
    let keyval = 0x31 + off as u32;
    assert!(h.key(keyval, 1 << 3, false), "Alt+数字应被吞掉");
    assert_eq!(
        h.pending_commit.take().unwrap(),
        gloss,
        "应上屏译文而非中文"
    );
}

#[test]
fn reset_cancels_pending_shift_tap() {
    // 按住 Shift（未夹别键）→ 切窗(reset)→ 松开 Shift：不应静默切中英。
    let mut h = sample_host();
    assert!(!h.engine.english_mode());
    h.key(0xffe1, 0, false); // Shift 按下，shift_armed = true
    h.reset(); // 切窗
    h.key(0xffe1, 1, true); // 在别处松开 Shift
    assert!(!h.engine.english_mode(), "切窗后松开 Shift 不应切换模式");
}

#[test]
fn english_mode_space_without_nav_commits_raw() {
    let mut h = sample_host();
    // Shift 轻点进英文模式
    h.key(0xffe1, 0, false);
    h.key(0xffe1, 1, true);
    assert!(h.engine.english_mode());
    type_str(&mut h, "kubectl"); // 词表里多半没有的词
    // 没动过高亮：空格应原样上屏所敲字母，不被英文候选替换；空格本身交给应用
    assert!(!h.key(0x20, 0, false));
    assert_eq!(h.pending_commit.take().unwrap(), "kubectl");
    assert!(!h.composing());
}

#[test]
fn page_indicator_multi_page() {
    let mut h = sample_host();
    // 找一个候选数超过一页的输入
    type_str(&mut h, "shi");
    if h.page_count() > 1 {
        assert_eq!(
            h.page_indicator(),
            format!("1/{}", h.page_count()),
            "首页应显示 1/N"
        );
        h.turn_page(1);
        assert_eq!(
            h.page_indicator(),
            format!("2/{}", h.page_count()),
            "翻页后应显示 2/N"
        );
    }
    // 清空后无候选：无页码
    h.reset();
    assert_eq!(h.page_indicator(), "", "无候选时不显示页码");
}

#[test]
fn default_page_keys_brackets_turn_pages() {
    // 缺省翻页键是配置的 `[` `]`(DEFAULT_PAGE_KEYS)，不是 - =。
    let mut h = sample_host();
    type_str(&mut h, "s");
    assert!(h.page_count() > 1, "输入 s 应有多页候选");
    assert!(h.key(0x5d, 0, false), "] 应被吞掉");
    assert_eq!(h.page, 1, "] 应翻到下一页");
    assert!(h.pending_commit.is_none(), "翻页不应上屏任何东西");
    assert!(h.key(0x5b, 0, false), "[ 应被吞掉");
    assert_eq!(h.page, 0, "[ 应翻回上一页");
    assert!(h.composing());
}

#[test]
fn page_keys_config_comma_period() {
    // 配置 page_keys = ",." 后，逗号句号翻页而不再当标点。
    let mut config = qingjian_platform::Config::default();
    config.general.page_keys = ",.".to_owned();
    let mut h = host_with(config);
    type_str(&mut h, "s");
    assert!(h.page_count() > 1);
    assert!(h.key(0x2e, 0, false), ". 应被吞掉");
    assert_eq!(h.page, 1, ". 应翻到下一页");
    assert!(h.pending_commit.is_none());
    assert!(h.key(0x2c, 0, false), ", 应被吞掉");
    assert_eq!(h.page, 0, ", 应翻回上一页");
}

#[test]
fn hyphen_enters_raw_segment_space_commits() {
    // 组句中敲 `-`：进英文直输段(no-way)，空格整段原样上屏、空格本身交给应用。
    let mut h = sample_host();
    type_str(&mut h, "ni");
    assert!(h.key(0x2d, 0, false), "- 应被吞掉(进缓冲区)");
    assert!(h.composing());
    assert!(h.engine.raw_mode(), "- 之后应是英文直输段");
    type_str(&mut h, "hao");
    assert!(
        !h.key(0x20, 0, false),
        "直输段的空格应透传(hello, world 的空格要在)"
    );
    assert_eq!(h.pending_commit.take().unwrap(), "ni-hao");
    assert!(!h.composing());
}

#[test]
fn raw_segment_takes_page_keys_and_punctuation() {
    // 直输段里可见字符一律追加：翻页键字符、标点都是字面。
    let mut h = sample_host();
    type_str(&mut h, "ni");
    assert!(h.key(0x2d, 0, false));
    assert!(h.key(0x5b, 0, false), "直输段里 [ 应进缓冲区而非翻页");
    assert!(h.key(0x2c, 0, false), "直输段里 , 应进缓冲区而非转全角");
    assert!(h.composing());
    assert!(h.pending_commit.is_none(), "追加过程不应上屏");
    assert!(h.key(0xff0d, 0, false), "回车整段原样上屏");
    assert_eq!(h.pending_commit.take().unwrap(), "ni-[,");
}

#[test]
fn delete_shortcut_swallowed_while_composing() {
    // 删词键（缺省 Shift+数字）：X 布局下 Shift+1 的 keysym 是 `!`，须映射回数字位。
    // 组句中按下：吞掉、不上屏、仍在组句（词库词没什么可删，但键不能漏给应用）。
    let mut h = sample_host();
    type_str(&mut h, "ni");
    assert!(h.key(0x21, 1, false), "组句中 Shift+1(!)应被删词键吞掉");
    assert!(h.pending_commit.is_none(), "删词不应上屏任何东西");
    assert!(h.composing(), "删词后仍在组句");
    h.key(0xff1b, 0, false);
    // 不组句时 Shift+1 还是标点：照走全角转换。
    assert!(h.key(0x21, 1, false), "空闲时 ! 应转全角并吞掉");
    assert_eq!(h.pending_commit.take().unwrap(), "\u{ff01}");
}

#[test]
fn alt_shift_digit_reaches_second_sense() {
    // 译词第二组（缺省 Alt+Shift+数字）：X 布局下 keysym 是符号，也得映射回数字位。
    let mut h = sample_host();
    type_str(&mut h, "ni");
    let alt_shift = (1 << 0) | (1 << 3);
    assert!(
        h.key(0x21, alt_shift, false),
        "组句中 Alt+Shift+1(!)应被译词键吞掉"
    );
    assert!(h.composing() || h.pending_commit.is_some());
}

#[test]
fn composing_swallows_unknown_editing_keys() {
    // 组句期间所有编辑动作都由我们接管；不认识的一律吞掉（macOS 同款），按下与松开对称。
    let mut h = sample_host();
    type_str(&mut h, "ni");
    assert!(h.key(0xff09, 0, false), "组句中 Tab 按下应被吞掉");
    assert!(h.key(0xff09, 0, true), "组句中 Tab 松开应被吞掉");
    assert!(h.key(0xff50, 0, false), "组句中 Home 应被吞掉");
    assert!(h.key(0xffbe, 0, false), "组句中 F1 应被吞掉");
    assert!(h.composing(), "吞掉之后组句不受影响");
    assert!(!h.key(0xffe3, 0, false), "Ctrl 修饰键本身按下应透传");
    assert!(!h.key(0xfe03, 0, false), "AltGr(ISO_Level3_Shift)应透传");
    assert!(!h.key(0xff7f, 0, false), "Num_Lock 应透传");
    assert!(!h.key(0x1008ff13, 0, false), "XF86 媒体键应透传");
    assert!(!h.key(0xe9, 0, false), "AltGr 打出的非 ASCII 字符(é)应透传");
    h.key(0xff1b, 0, false);
    assert!(!h.key(0xff09, 0, false), "空闲时 Tab 应透传");
}

#[test]
fn english_mode_punctuation_stays_half_width() {
    // 英文模式标点一律半角（macOS 口径）：不转全角，原样透传；[ ] 也不当翻页键。
    let mut h = sample_host();
    h.key(0xffe1, 0, false);
    h.key(0xffe1, 1, true);
    assert!(h.engine.english_mode());
    // 空闲：透传，不出「【」「，」
    assert!(!h.key(0x5b, 0, false), "英文模式空闲 [ 应透传半角");
    assert!(h.pending_commit.is_none(), "不应转出全角「【」");
    assert!(!h.key(0x2c, 0, false), "英文模式空闲 , 应透传半角");
    assert!(h.pending_commit.is_none());
    // 组句中：先把敲的字母原样上屏，标点本身交给应用；] 不翻页
    type_str(&mut h, "kubectl");
    assert!(!h.key(0x5d, 0, false), "英文模式组句中 ] 应透传而非翻页");
    assert_eq!(
        h.pending_commit.take().unwrap(),
        "kubectl",
        "敲的字母应原样上屏"
    );
    assert!(!h.composing());
}

#[test]
fn expression_mode_takes_digits_and_operators() {
    // v1+2：表达式模式里数字与运算符进缓冲区，不当选词键/标点；候选出 3。
    let mut h = sample_host();
    type_str(&mut h, "v");
    assert!(h.engine.expression_mode(), "v 应进入表达式模式");
    assert!(h.key(0x31, 0, false), "表达式里 1 应进缓冲区");
    assert!(h.key(0x2b, 1, false), "+(Shift+=)应进缓冲区");
    assert!(h.key(0x32, 0, false), "表达式里 2 应进缓冲区");
    assert!(h.pending_commit.is_none(), "追加过程不上屏");
    let has_result = (0..h.layout.len())
        .filter_map(|i| h.layout.candidate(i))
        .any(|c| c.text == "3");
    assert!(has_result, "v1+2 的候选里应有 3");
    // 表达式里 Shift+( 是括号，不当删词/译词快捷键
    assert!(h.key(0x28, 1, false), "( 应进缓冲区");
    assert!(h.composing());
    assert!(h.pending_commit.is_none(), "( 不应触发删词或上屏");
}

#[test]
fn unicode_entry_takes_digits() {
    // u4e00：问字模式的码点输入，数字进缓冲区，候选出「一」。
    let mut h = sample_host();
    type_str(&mut h, "u");
    assert!(h.engine.question_mode(), "u 应进入问字模式");
    assert!(h.key(0x34, 0, false), "码点里 4 应进缓冲区");
    type_str(&mut h, "e");
    assert!(h.key(0x30, 0, false), "码点里 0 应进缓冲区");
    assert!(h.key(0x30, 0, false));
    let has_char = (0..h.layout.len())
        .filter_map(|i| h.layout.candidate(i))
        .any(|c| c.text == "一");
    assert!(has_char, "u4e00 的候选里应有「一」");
}

#[test]
fn correction_gives_intended_candidate() {
    // 拼写纠错（引擎侧，验证 Linux 按键路径畅通）：nihoa 应仍给出「你好」。
    let mut h = sample_host();
    type_str(&mut h, "nihoa");
    let has = (0..h.layout.len())
        .filter_map(|i| h.layout.candidate(i))
        .any(|c| c.text == "你好");
    assert!(has, "nihoa 应纠错给出「你好」");
}

#[test]
fn plain_digit_still_selects_chinese() {
    // 不带修饰键的数字仍选中文，不被译词快捷键抢走。
    let mut h = sample_host();
    type_str(&mut h, "ni");
    let first = h.layout.candidate(0).map(|c| c.text.clone());
    assert!(h.key(0x31, 0, false));
    if let Some(expected) = first {
        assert_eq!(h.pending_commit.take().unwrap(), expected);
    }
}

#[test]
fn arrow_keys_follow_documented_semantics() {
    // 与用户文档/macOS 对齐：←/→=移动拼音光标（候选按光标前拼音算），↑/↓=移动高亮，
    // Home/End=光标到开头/末尾。此前 Linux 壳把 ←/→ 错接成移高亮、↑/↓ 错接成翻页。
    let mut h = sample_host();
    type_str(&mut h, "nihao");
    let end = h.preedit_cursor;
    assert!(h.key(0xff51, 0, false), "← 应被吞");
    assert!(h.preedit_cursor < end, "← 应左移拼音光标");
    assert!(h.key(0xff50, 0, false), "Home 应被吞");
    assert_eq!(h.preedit_cursor, 0, "Home 光标应到开头");
    assert!(h.key(0xff57, 0, false), "End 应被吞");
    assert_eq!(h.preedit_cursor, end, "End 应回到末尾");
    if h.layout.len() > 1 {
        let hl = h.highlighted;
        assert!(h.key(0xff54, 0, false), "↓ 应被吞");
        assert_eq!(h.highlighted, hl + 1, "↓ 应下移高亮");
        assert!(h.key(0xff52, 0, false), "↑ 应被吞");
        assert_eq!(h.highlighted, hl, "↑ 应移回");
    }
}

#[test]
fn fix_earlier_syllable_with_alt_arrows() {
    // 用户文档场景：前面音节敲错 → ⌥+← 按音节移到其后 → ⌥+Backspace 删掉重输 → ⌘+→ 回末尾。
    // Linux 对应 Alt 与 Super。
    let mut h = sample_host();
    type_str(&mut h, "nihao");
    let alt = 1 << 3;
    let super_ = 1 << 6;
    assert!(h.key(0xff51, alt, false), "Alt+← 应被吞(按音节左移)");
    assert!(
        h.key(0xff08, alt, false),
        "Alt+Backspace 应被吞(删光标前音节)"
    );
    assert_eq!(h.engine.composition().text(), "hao", "ni 应被整音节删掉");
    type_str(&mut h, "ni");
    assert_eq!(
        h.engine.composition().text(),
        "nihao",
        "光标处插入重输的音节"
    );
    assert!(h.key(0xff53, super_, false), "Super+→ 应被吞(光标到末尾)");
    let text_len = h.engine.composition().text().len();
    assert_eq!(h.engine.composition().cursor(), text_len, "光标应在末尾");
    // Super+Backspace：删光标前全部。
    assert!(h.key(0xff08, super_, false));
    assert!(!h.composing(), "Super+Backspace 应清掉全部拼音");
}

#[test]
fn english_mode_space_reaches_app_after_commit() {
    // 英文模式空格提交后，空格本身要交给应用（macOS/Windows 同款口径），
    // 否则英文单词之间打不出空格。中文模式空格仍是选词键、照吞。
    let mut h = sample_host();
    h.key(0xffe1, 0, false);
    h.key(0xffe1, 1, true);
    assert!(h.engine.english_mode());
    // 没动过高亮：原样上屏所敲字母，空格透传
    type_str(&mut h, "kubectl");
    assert!(!h.key(0x20, 0, false), "英文模式空格不该被吞");
    assert_eq!(h.pending_commit.take().unwrap(), "kubectl");
    assert!(!h.composing());
    // 动过高亮：选高亮词，空格同样透传
    type_str(&mut h, "helo");
    h.key(0xff52, 0, false);
    assert!(h.navigated);
    assert!(!h.key(0x20, 0, false), "选词后的空格也不该被吞");
    assert!(h.pending_commit.take().is_some());
    assert!(!h.composing());
}

#[test]
fn tab_turns_page_in_chinese_mode() {
    // 组句中 Tab 翻下一页、⇧+Tab(ISO_Left_Tab 0xfe20)翻上一页
    // （macOS 口径；Linux 无整句补全，「有补全先接受」那臂用不上）。
    let mut config = qingjian_platform::Config::default();
    config.general.page_size = 2;
    let mut h = host_with(config);
    type_str(&mut h, "shi");
    assert!(h.page_count() > 1, "page_size=2 下 shi 应有多页候选");
    assert!(h.key(0xff09, 0, false), "Tab 应被吞");
    assert_eq!(h.page, 1, "Tab 应翻到下一页");
    assert!(h.key(0xfe20, 1, false), "⇧+Tab 应被吞");
    assert_eq!(h.page, 0, "⇧+Tab 应翻回上一页");
    assert!(h.key(0xfe20, 1, true), "⇧+Tab 的松键应销账吞掉");
    assert!(h.composing(), "翻页不该结束组句");
}

#[test]
fn tab_commits_highlighted_in_english_mode() {
    // 英文模式 Tab 选中高亮的词（macOS 口径）。
    let mut h = sample_host();
    h.key(0xffe1, 0, false);
    h.key(0xffe1, 1, true);
    type_str(&mut h, "helo");
    assert!(h.key(0xff09, 0, false), "英文模式 Tab 应被吞");
    assert!(h.pending_commit.take().is_some(), "Tab 应选中高亮词上屏");
    assert!(!h.composing());
}
