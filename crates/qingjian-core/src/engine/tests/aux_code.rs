//! 辅码态：触发消歧、逐键即筛、删码回退、上屏收尾与 preedit 分段。

use qingjian_dictionary::AuxCodeTable;

use super::*;

/// 测试用的小码表：只覆盖样例词库里 `kaifa` 能查出来的那几条。
/// 码长各不相同（kf / kfz / kh），重排与逐键收窄的期望才好写。
fn codes() -> Arc<dyn AuxCodeLookup> {
    Arc::new(
        AuxCodeTable::from_pairs([
            ("开发".to_owned(), "kf".to_owned()),
            ("开发者".to_owned(), "kfz".to_owned()),
            ("开".to_owned(), "kh".to_owned()),
        ])
        .unwrap(),
    )
}

fn aux_engine() -> Engine {
    let mut engine = engine().with_aux_codes(vec![codes()]);
    // 测试都按「辅码开着且有码表」的常规路径来；开关本身的边界见下面的专项测试
    engine.set_aux_enabled(true);
    // 这些用例写的是「码段只加权、不筛候选」的语义，开关缺省是关的，这里显式打开
    engine.set_aux_weighting(true);
    engine
}

/// 缺省配置（`[general] aux_code_weighting` 关着）：码段是过滤器，对不上的词不出。
fn filtering_aux_engine() -> Engine {
    let mut engine = engine().with_aux_codes(vec![codes()]);
    engine.set_aux_enabled(true);
    engine
}

fn candidates(engine: &Engine) -> Vec<String> {
    engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .map(|c| c.text)
        .collect()
}

/// 全拼能完整切分、光标在段尾 → 进辅码态，码段空、候选不变，preedit 多一个 `;`。
#[test]
fn trigger_enters_aux_without_filtering() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    let before = candidates(&engine);
    assert!(engine.aux_trigger(';'));
    engine.enter_aux();
    assert!(engine.in_aux());
    assert_eq!(engine.aux_code(), "");
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "kai'fa;");
    assert_eq!(query.marked_cursor(), 7);
    assert_eq!(
        query
            .marked_segments()
            .iter()
            .map(|s| (s.text.as_str(), s.kind))
            .collect::<Vec<_>>(),
        [("kai'fa", MarkedKind::Typed), (";", MarkedKind::Typed)]
    );
    let after: Vec<String> = query
        .candidates
        .items
        .iter()
        .map(|c| c.text.clone())
        .collect();
    assert_eq!(before, after);
    assert!(after.contains(&"开发者".to_owned()));
}

/// 微软 / 搜狗双拼里能当韵母的 `;` 优先当韵母（`x;` = xing），不进辅码态。
#[test]
fn semicolon_as_a_final_beats_the_trigger() {
    let mut engine = aux_engine();
    engine.set_shuangpin(Some(Scheme::Microsoft));
    engine.set_input("x");
    assert!(engine.takes_semicolon());
    assert!(!engine.aux_trigger(';'));
    engine.push(';');
    assert_eq!(engine.query().unwrap().marked_text(), "xing");
    // 音节已经打完时 `;` 不再是韵母键，可以触发
    engine.set_input("ni");
    assert!(!engine.takes_semicolon());
    assert!(engine.aux_trigger(';'));
}

/// 简拼、没打完的音节也能进辅码（`nh;ni` 这种）：码段只加权、不筛候选，进不去比进去更糟。
/// 光标停在拼音段中间时不触发 —— 作用域不完整，壳只按光标前那段算候选。
#[test]
fn abbreviations_and_partial_syllables_still_trigger() {
    let mut engine = aux_engine();
    for text in ["kf", "kaif", "nh"] {
        engine.set_input(text);
        assert!(engine.aux_trigger(';'), "{text} 应该能进辅码");
    }
    engine.set_input("kaifa");
    engine.move_cursor_left();
    assert!(!engine.aux_trigger(';'));
    engine.move_cursor_end();
    assert!(engine.aux_trigger(';'));
    // 触发键之外的键不触发
    assert!(!engine.aux_trigger(','));
}

/// 没打完的音节进去以后码段照常收，候选一条不少（`kaif;ni` 还是原来那批，对不上码只往后挪）。
#[test]
fn a_code_after_a_partial_syllable_keeps_every_candidate() {
    let mut engine = aux_engine();
    engine.set_input("kaif");
    let before = candidates(&engine);
    assert!(!before.is_empty());
    assert!(engine.aux_trigger(';'));
    engine.enter_aux();
    assert!(engine.push_aux_code('n'));
    assert!(engine.push_aux_code('i'));
    assert_eq!(engine.aux_code(), "ni");
    assert!(engine.query().unwrap().marked_text().ends_with(";ni"));
    assert_eq!(candidates(&engine).len(), before.len(), "码段不筛候选");
}

/// 码段每多一个字母就重筛一次（无码词直接隐藏，不是降权）。
/// 码长不同的一批：`k` 三条都在，`kf` 里完全匹配的 开发 排第一，`kfh` 筛空。
#[test]
fn each_code_letter_narrows_the_candidates() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    engine.enter_aux();
    assert!(engine.push_aux_code('k'));
    // 都不是完全匹配，按码长降序：kfz(3) 的 开发者 最先，剩下两条同长保持原序
    assert_eq!(
        candidates(&engine),
        ["开发者", "开发", "开", "开放", "开饭"]
    );
    assert!(engine.push_aux_code('f'));
    assert_eq!(engine.aux_code(), "kf");
    assert_eq!(
        candidates(&engine),
        ["开发", "开发者", "开放", "开饭", "开"]
    );
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "kai'fa;kf");
    assert_eq!(
        query
            .marked_segments()
            .iter()
            .map(|s| (s.text.as_str(), s.kind))
            .collect::<Vec<_>>(),
        [
            ("kai'fa", MarkedKind::Typed),
            (";", MarkedKind::Typed),
            ("kf", MarkedKind::AuxCode)
        ]
    );
    // 命中的那条码随候选一起给壳（显示码开关打开时用）
    assert_eq!(query.candidates.items[0].aux_code.as_deref(), Some("kf"));
}

/// 辅码态只出命中码的词：没有码的候选（自定义短语、快捷、整句、emoji）一律不出；
/// 筛空时候选为空、preedit 仍在（壳据此收起候选窗、只留拼音行）。
#[test]
fn candidates_without_a_code_are_hidden() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    engine.enter_aux();
    engine.push_aux_code('k');
    engine.push_aux_code('h');
    assert_eq!(
        candidates(&engine),
        ["开", "开发", "开放", "开饭", "开发者"]
    );
    engine.push_aux_code('z');
    let query = engine.query().unwrap();
    assert!(!query.candidates.items.is_empty(), "码段对不上也不该空");
    assert_eq!(query.marked_text(), "kai'fa;khz");
}

/// 首条码的挂载看显示开关：纯拼音态且 `aux_code_show` 关（缺省）时**不逐候查码**，
/// 候选全不带 `aux_code`（省掉单次查询 500 条 × 表数的二分）；`set_aux_show(true)` 后回来。
#[test]
fn pure_pinyin_skips_first_code_unless_shown() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    let query = engine.query().unwrap();
    assert!(query.candidates.items.iter().all(|c| c.aux_code.is_none()));

    engine.set_aux_show(true);
    let query = engine.query().unwrap();
    // 测试码表只有 开发=kf、开发者=kfz、开=kh：其余候选一律不带码
    for candidate in &query.candidates.items {
        let expected = match candidate.text.as_str() {
            "开发" => Some("kf"),
            "开发者" => Some("kfz"),
            "开" => Some("kh"),
            _ => None,
        };
        assert_eq!(
            candidate.aux_code.as_deref(),
            expected,
            "{}",
            candidate.text
        );
    }
    assert!(query.candidates.items.iter().any(|c| c.text == "开发"));
}

/// 辅码态空码段（刚触发 / 删空停住）不看显示开关：还是挂首条码——进辅码态看码有引导意义，
/// 按下触发键注记不消失；筛码时仍是命中码。
#[test]
fn empty_code_segment_keeps_the_first_code_regardless_of_show() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    engine.enter_aux();
    let query = engine.query().unwrap();
    assert_eq!(
        query
            .candidates
            .items
            .iter()
            .find(|c| c.text == "开发")
            .unwrap()
            .aux_code
            .as_deref(),
        Some("kf")
    );
}

/// 退格删码段、逐键放宽；删空停在辅码态（`aux_code_keep_empty` 缺省开）——
/// `;` 仍在、无码词全部回来，空码段再按一次退格才退出、拼音一字不动。
#[test]
fn backspace_widens_then_stays_in_aux_mode() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    let full = candidates(&engine);
    engine.enter_aux();
    engine.push_aux_code('k');
    engine.push_aux_code('f');
    engine.push_aux_code('h');
    assert!(!candidates(&engine).is_empty(), "码段对不上也不该空");
    assert!(engine.backspace(false).consumed());
    assert_eq!(engine.aux_code(), "kf");
    assert_eq!(
        candidates(&engine),
        ["开发", "开发者", "开放", "开饭", "开"]
    );
    assert!(engine.backspace(false).consumed());
    assert_eq!(engine.aux_code(), "k");
    assert_eq!(
        candidates(&engine),
        ["开发者", "开发", "开", "开放", "开饭"]
    );
    // 删到空：停在辅码态，`;` 仍在 preedit，候选全回来
    assert!(engine.backspace(false).consumed());
    assert!(engine.in_aux());
    assert_eq!(engine.aux_code(), "");
    assert_eq!(engine.query().unwrap().marked_text(), "kai'fa;");
    assert_eq!(candidates(&engine), full);
    // 再敲字母重新筛——辅码态还活着
    assert!(engine.push_aux_code('k'));
    assert_eq!(
        candidates(&engine),
        ["开发者", "开发", "开", "开放", "开饭"]
    );
    assert!(engine.backspace(false).consumed());
    assert!(engine.in_aux());
    // 空码段再按退格：退出辅码态，拼音一字不动
    assert!(engine.backspace(false).consumed());
    assert!(!engine.in_aux());
    assert_eq!(engine.query().unwrap().marked_text(), "kai'fa");
    assert_eq!(candidates(&engine), full);
    assert_eq!(engine.composition().text(), "kaifa");
}

/// `aux_code_keep_empty = false` 删空即回拼音态（开关关掉的旧行为）。
#[test]
fn backspace_leaves_aux_mode_when_keep_empty_is_off() {
    let mut engine = aux_engine();
    engine.set_aux_keep_empty(false);
    engine.set_input("kaifa");
    let full = candidates(&engine);
    engine.enter_aux();
    engine.push_aux_code('k');
    engine.push_aux_code('f');
    assert!(engine.backspace(false).consumed());
    assert_eq!(engine.aux_code(), "k");
    // 删到空：回拼音态，`;` 从 preedit 消失，候选全回来
    assert!(engine.backspace(false).consumed());
    assert!(!engine.in_aux());
    assert_eq!(engine.query().unwrap().marked_text(), "kai'fa");
    assert_eq!(candidates(&engine), full);
    assert_eq!(engine.composition().text(), "kaifa");
}

/// 刚触发（码段空）就退格 = 退出辅码态、拼音一个字符都不动。
#[test]
fn backspace_on_an_empty_code_leaves_the_pinyin_alone() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    let full = candidates(&engine);
    engine.enter_aux();
    assert!(engine.backspace(false).consumed());
    assert!(!engine.in_aux());
    assert_eq!(engine.composition().text(), "kaifa");
    assert_eq!(candidates(&engine), full);
}

/// 拼音态退格删拼音段末字符。
#[test]
fn backspace_in_pinyin_state_deletes_a_letter() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    assert!(engine.backspace(false).consumed());
    assert_eq!(engine.composition().text(), "kaif");
}

/// Esc 在辅码态清码段（拼音与候选保持），在拼音态清拼音。
#[test]
fn escape_clears_the_code_then_the_pinyin() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    let full = candidates(&engine);
    engine.enter_aux();
    engine.push_aux_code('k');
    engine.clear_aux();
    assert!(!engine.in_aux());
    assert_eq!(engine.composition().text(), "kaifa");
    assert_eq!(candidates(&engine), full);
    engine.clear();
    assert!(engine.composition().is_empty());
}

/// 辅码态选词上屏，码段清空回初始态；学习按拼音段的全拼记，码段与触发键不在键里。
#[test]
fn committing_from_aux_records_the_choice_under_the_pinyin() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    engine.enter_aux();
    engine.push_aux_code('k');
    engine.push_aux_code('f');
    let candidate = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(candidate.text, "开发");
    engine.commit(&candidate);
    assert!(!engine.in_aux());
    assert!(engine.composition().is_empty());
    let last = engine.recent_commits.last().unwrap();
    assert!(last.same_input("kaifa"));
    assert_eq!(last.text, "开发");
}

/// 标点先上屏高亮候选（壳做），再走组句外标点语义；码段随之清空。
#[test]
fn punctuation_ends_the_aux_state() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    engine.enter_aux();
    engine.push_aux_code('k');
    assert_eq!(engine.punctuate(';'), Some("；"));
    assert!(!engine.in_aux());
}

/// 回车原样上屏拼音段，码段与触发键都不跟着上屏。
#[test]
fn take_raw_keeps_the_code_out_of_the_text() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    engine.enter_aux();
    engine.push_aux_code('k');
    engine.push_aux_code('h');
    assert_eq!(engine.take_raw(), "kaifa");
    assert!(!engine.in_aux());
    assert!(engine.composition().is_empty());
}

/// 翻页键与 `'` 不进码段，壳按原语义处理。
#[test]
fn paging_keys_and_apostrophes_do_not_enter_the_code() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    engine.enter_aux();
    for key in ['[', ']', ',', '.', '\'', '1', 'A'] {
        assert!(!engine.push_aux_code(key), "{key}");
    }
    assert_eq!(engine.aux_code(), "");
}

/// 辅码态里再敲触发键是幂等的。
#[test]
fn triggering_again_is_idempotent() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    engine.enter_aux();
    engine.push_aux_code('k');
    assert!(!engine.aux_trigger(';'));
    engine.enter_aux();
    assert_eq!(engine.aux_code(), "k");
}

/// 重排：完全匹配码 > 码长降序 > 原词频序。
#[test]
fn exact_code_matches_come_first_and_longer_codes_before_shorter_ones() {
    let table = Arc::new(
        AuxCodeTable::from_pairs([
            ("开".to_owned(), "kf".to_owned()),
            ("开发".to_owned(), "kfa".to_owned()),
            ("开放".to_owned(), "kfb".to_owned()),
        ])
        .unwrap(),
    );
    let mut engine = engine().with_aux_codes(vec![table]);
    engine.set_input("kai");
    engine.enter_aux();
    engine.push_aux_code('k');
    engine.push_aux_code('f');
    let query = engine.query().unwrap();
    let texts: Vec<&str> = query
        .candidates
        .items
        .iter()
        .map(|c| c.text.as_str())
        .collect();
    // 开 的码正好是 kf（完全匹配），排在两个前缀命中的三段码之前
    assert_eq!(texts.first(), Some(&"开"));
    assert!(texts.contains(&"开发") && texts.contains(&"开放"));
    assert_eq!(query.candidates.items[0].aux_code.as_deref(), Some("kf"));
    // 码段只调顺序：`kai` 这套切分本来就只有这三个词，一条都不少
    assert_eq!(query.candidates.items.len(), 3);
}

/// 开关三态之一：开着但没挂码表也不触发（坏文件全被装配层跳过后同样归并到这里），
/// `;` 完全保持原生行为；表来了才触发。
#[test]
fn without_any_table_the_trigger_stays_off() {
    let mut bare = engine();
    bare.set_aux_enabled(true);
    bare.set_input("kaifa");
    assert!(!bare.aux_trigger(';'));
    bare.set_aux_codes(vec![codes()]);
    assert!(bare.aux_trigger(';'));
}

/// 开关三态之二：总开关缺省关（`[aux_code] enabled`）——装了码表也不触发，
/// 开了才生效；关着时纯拼音态也不逐候查首条码（见 `pure_pinyin_skips_first_code_unless_shown`）。
#[test]
fn the_enabled_gate_defaults_off() {
    let mut engine = engine().with_aux_codes(vec![codes()]);
    engine.set_input("kaifa");
    assert!(!engine.aux_trigger(';'));
    engine.set_aux_enabled(true);
    assert!(engine.aux_trigger(';'));
}

/// 触发键校验：单字符标点、非字母数字、非 `'`（拼音隔音符）、非**当前**翻页键——
/// `[ ]`、`,.` 只有真配成 `[general] page_keys` 才拒，换一套就放行。
#[test]
fn trigger_key_validation() {
    let paging = ('[', ']');
    assert!(is_valid_aux_code_key(';', paging));
    assert!(!is_valid_aux_code_key('\'', paging));
    assert!(!is_valid_aux_code_key('a', paging));
    assert!(!is_valid_aux_code_key('1', paging));
    assert!(!is_valid_aux_code_key('中', paging));
    // 翻页键两态：配成 [ ] 时 [ ] 拒；配成 ,. 时 [ ] 放行、, . 拒
    assert!(!is_valid_aux_code_key('[', paging));
    assert!(!is_valid_aux_code_key(']', paging));
    assert!(is_valid_aux_code_key('[', (',', '.')));
    assert!(is_valid_aux_code_key('.', ('[', ']')));
    assert!(!is_valid_aux_code_key('.', (',', '.')));
    let mut engine = aux_engine();
    assert_eq!(engine.aux_code_key(), ';');
    engine.set_aux_code_key('a', paging);
    assert_eq!(engine.aux_code_key(), ';');
    engine.set_aux_code_key('/', paging);
    assert_eq!(engine.aux_code_key(), '/');
    engine.set_input("kaifa");
    assert!(engine.aux_trigger('/'));
    assert!(!engine.aux_trigger(';'));
}

/// 输入日志按实际敲键记：拼音 + 触发键 + 码段一条键串，replay 逐键重喂不需要特殊逻辑；
/// 学习键仍是拼音段的全拼，码段与触发键不在里面。
#[test]
fn the_input_log_records_the_trigger_and_the_code() {
    let entries = Arc::new(Mutex::new(Vec::new()));
    let mut engine = aux_engine().with_input_logger(Box::new(MemoryLogger(entries.clone())));
    engine.set_input("kaifa");
    engine.enter_aux();
    engine.push_aux_code('k');
    engine.push_aux_code('f');
    let candidate = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(candidate.text, "开发");
    engine.commit(&candidate);
    let entries = entries.lock().unwrap();
    let InputLogEntry::Commit(commit) = &entries[0] else {
        panic!("expected a commit");
    };
    assert_eq!(commit.keys, "kaifa;kf");
    assert_eq!(commit.scope, "kaifa");
    assert_eq!(commit.pinyin, "kai'fa");
}

/// 码段一个码都对不上时，把它当成「要哪个字」的提示：拆字能指到那个字的候选排前面。
///
/// `chefu` 拆成 车 + 甫 得到「辅」。样例词库里没有含「辅」的词，所以这里验的是两件事：
/// 码段确实进了辅码态（没有整串上屏），以及提示字确实算出来了。
#[test]
fn a_code_that_matches_nothing_is_read_as_a_hint() {
    let mut engine = aux_engine();
    assert!(engine.hint_chars("chefu").contains(&'辅'), "拆字该指到 辅");

    engine.set_input("kaifa");
    engine.enter_aux();
    for ch in "chefu".chars() {
        assert!(engine.push_aux_code(ch), "码段该能敲进去：{ch}");
    }
    let query = engine.query().unwrap();
    let texts: Vec<String> = query
        .candidates
        .items
        .iter()
        .map(|c| c.text.clone())
        .collect();
    assert!(
        texts.iter().all(|text| !text.contains(';')),
        "码段该进辅码态，不该整串上屏：{texts:?}"
    );
    // 样例词库里没有含「辅」的词，这一列本来就是空的；这里只验码段进了辅码态
}

/// 辅码态里把光标挪到拼音中间再退格：删的是光标前那个字母，码段原样留着。
///
/// 敲错的是前面某个字母时，谁也不想为了改它先退出辅码态、重打一遍码。
#[test]
fn backspace_at_the_caret_edits_the_letter_not_the_code() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    engine.enter_aux();
    engine.push_aux_code('k');
    engine.push_aux_code('f');
    // 方向键把光标挪到 f 之前（辅码态里点拼音行不挪光标，见 `caret.rs`）
    assert!(engine.move_cursor_left());
    assert!(engine.move_cursor_left());
    assert!(engine.backspace(false).consumed());
    // 码段一个字都没少，删的是光标前那个 i
    assert!(engine.in_aux());
    assert_eq!(engine.aux_code(), "kf");
    assert_eq!(engine.composition().text(), "kafa");
    // 光标前还有字母就继续删字母（a、k），删光了停在行首
    assert!(engine.backspace(false).consumed());
    assert!(engine.backspace(false).consumed());
    assert_eq!(engine.composition().text(), "fa");
    // 光标在行首、前面没有字母：这一记退格不吃键，码段也不动
    assert!(!engine.backspace(false).consumed());
    assert_eq!(engine.aux_code(), "kf");
    // 光标回到末尾，退格才开始删码段
    engine.move_cursor_end();
    assert!(engine.backspace(false).consumed());
    assert_eq!(engine.aux_code(), "k");
}

/// 缺省（开关关着）码段仍然是过滤器：对不上码的词一条都不出，与辅码一直以来的行为一致。
#[test]
fn without_the_switch_the_code_still_filters() {
    let mut engine = filtering_aux_engine();
    engine.set_input("kaifa");
    engine.enter_aux();
    for key in "kf".chars() {
        engine.push_aux_code(key);
    }
    let texts = candidates(&engine);
    assert!(!texts.is_empty(), "码段对得上的时候照常出");
    assert!(
        texts.iter().all(|text| text != "开放" && text != "开"),
        "对不上 `kf` 的词不该出现：{texts:?}"
    );
}

/// 开关关着时，码段一个都对不上就是没有候选（不会凭空拼出「开粗」这种词）。
#[test]
fn without_the_switch_nothing_is_synthesised() {
    let mut engine = filtering_aux_engine();
    engine.set_input("kaifa");
    engine.enter_aux();
    for key in "zz".chars() {
        engine.push_aux_code(key);
    }
    assert!(candidates(&engine).is_empty(), "码段对不上时不该有候选");
}
