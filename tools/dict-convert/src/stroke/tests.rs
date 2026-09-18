//! stroke 子命令的规则与抽样对照测试。

use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::path::Path;

use crate::stroke::normalize;
use crate::stroke::rules::PrcRules;
use crate::stroke::verify::{compare, sample};

/// 覆盖表里的四类锚点各来一条，看改写落在哪里。
#[test]
fn rewrites_follow_anchor() {
    let table = "rule\tcao\t2112\t122\tany\n\
                 rule\tzou\t4554\t454\tsuffix\n\
                 rule\tfu\t552\t55\tboth\n\
                 rule\tyi\t413234\t45234\tprefix\n";
    let rules = PrcRules::parse(Path::new("test"), Cursor::new(table)).unwrap();
    // any 取最左一处，不在开头也命中（佳 这种跨部件边界的模式由 skip 挡）
    assert_eq!(rules.apply('花', "21123215"), "1223215");
    assert_eq!(rules.apply('佳', "32121121"), "3211221");
    // suffix 与 both 先看结尾，再退回开头
    assert_eq!(rules.apply('边', "534554"), "53454");
    assert_eq!(rules.apply('邓', "54552"), "5455");
    assert_eq!(rules.apply('阴', "5523511"), "553511");
    // prefix
    assert_eq!(rules.apply('补', "41323424"), "4523424");
}

/// 例外字跳过该规则，别的规则照常生效。
#[test]
fn skip_exempts_a_character() {
    let table = "rule\tcao\t2112\t122\tany\nskip\tcao\t卡\nrule\tzhi\t4134\t454\tsuffix\n";
    let rules = PrcRules::parse(Path::new("test"), Cursor::new(table)).unwrap();
    assert_eq!(rules.apply('卡', "21124"), "21124");
    assert_eq!(rules.apply('芝', "21124134"), "122454");
}

/// 整字覆盖优先于所有规则。
#[test]
fn override_wins_over_rules() {
    let table = "rule\tcao\t2112\t122\tany\nchar\t闹\t42541252\n";
    let rules = PrcRules::parse(Path::new("test"), Cursor::new(table)).unwrap();
    assert_eq!(rules.apply('闹', "1"), "42541252");
    assert_eq!(rules.rule_count(), 1);
    assert_eq!(rules.override_count(), 1);
}

/// 认不出的行与认不出的锚点直接报错，不静默吞掉。
#[test]
fn rejects_bad_lines() {
    for table in [
        "rule\tcao\t2112\t122\tmiddle\n",
        "skip\tcao\t卡\n",
        "rule\tcao\t2112\t122\tany\nchar\t闹\t4x\n",
        "nonsense\n",
    ] {
        assert!(
            PrcRules::parse(Path::new("test"), Cursor::new(table)).is_err(),
            "应当报错：{table}"
        );
    }
}

/// 点与捺合并成 n，1 2 3 5 原样保留。
#[test]
fn normalize_merges_dot_and_na() {
    assert_eq!(normalize("12345"), "123n5");
    assert_eq!(normalize("4134"), "n13n");
}

/// 抽样按表序每 stride 字取一个。
#[test]
fn sample_picks_every_nth() {
    let chars: Vec<char> = "一二三四五六七".chars().collect();
    assert_eq!(sample(&chars, 3), vec!['三', '六']);
    assert!(sample(&chars, 8).is_empty());
}

/// 对照只放白名单之外的不过。
#[test]
fn compare_flags_only_unwhitelisted() {
    let sampled: Vec<char> = "一瓦卸".chars().collect();
    let produced: HashMap<char, usize> = [('一', 1), ('瓦', 5), ('卸', 8)].into_iter().collect();
    let expected: HashMap<char, usize> = [('一', 1), ('瓦', 4), ('卸', 9)].into_iter().collect();
    let whitelist: HashSet<char> = ['瓦'].into_iter().collect();
    let (count, used, unmatched) = compare(&sampled, &produced, &expected, &whitelist);
    assert_eq!(count, 3);
    assert_eq!(used, whitelist);
    assert_eq!(unmatched, 1);
}
