//! 多音字姓氏的读音：名字以这些字开头时按姓读，不按这个字最常见的读音（曾 读 zeng 不读 ceng）。
//!
//! 只收作姓时的读音与单字表（`data/tones.tsv`）里最常见读音不同、或单字表里没有的姓；复姓排在前面先匹配。

const SURNAMES: &[(&str, &str)] = &[
    ("尉迟", "yu chi"),
    ("万俟", "mo qi"),
    ("单于", "chan yu"),
    ("长孙", "zhang sun"),
    ("澹台", "tan tai"),
    ("曾", "zeng"),
    ("单", "shan"),
    ("解", "xie"),
    ("区", "ou"),
    ("查", "zha"),
    ("仇", "qiu"),
    ("朴", "piao"),
    ("盖", "ge"),
    ("种", "chong"),
    ("召", "shao"),
    ("缪", "miao"),
    ("繁", "po"),
    ("秘", "bi"),
    ("员", "yun"),
    ("句", "gou"),
    ("柏", "bai"),
    ("折", "she"),
    ("翟", "zhai"),
    ("冼", "xian"),
];

/// `name` 开头的多音字姓：(姓占几个字, 读音)。
pub(super) fn surname(name: &str) -> Option<(usize, &'static str)> {
    SURNAMES
        .iter()
        .find(|(surname, _)| name.starts_with(surname) && name.len() > surname.len())
        .map(|(surname, reading)| (surname.chars().count(), *reading))
}
