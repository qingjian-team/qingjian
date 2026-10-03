/// 一套双拼方案的键位表。声母键里只列与字母本身不同的翘舌声母，
/// 其余辅音键（含 `y` `w`）就是自己；非声母键不在此列。
pub struct Table {
    /// 翘舌声母映射（键 → 声母）。
    pub digraph_initials: &'static [(char, &'static str)],

    /// 韵母键 → 可能的韵母，按优先级排（同一键配同一声母能拼出两个合法音节时取前面的，如 `lve` 先于 `lue`）。
    pub finals: &'static [(char, &'static [&'static str])],

    /// 零声母音节 → 两键写法（可以有多种，第一种是主写法）。
    pub zero_initials: &'static [(&'static str, &'static [&'static str])],

    /// 是否用到 `;` 键（微软 / 搜狗的 ing）。
    pub semicolon: bool,
}

/// 三个占键的翘舌声母：小鹤、自然码、微软、搜狗一致。
pub const DIGRAPH_INITIALS: [(char, &str); 3] = [('v', "zh"), ('i', "ch"), ('u', "sh")];

/// 小浪双拼的翘舌声母：e 为 zh，i 为 ch，v 为 sh。
pub const XIAOLANG_DIGRAPH_INITIALS: [(char, &str); 3] = [('e', "zh"), ('i', "ch"), ('v', "sh")];

/// 智能 ABC 的翘舌声母：a 为 zh，e 为 ch，v 为 sh。
pub const ABC_DIGRAPH_INITIALS: [(char, &str); 3] = [('a', "zh"), ('e', "ch"), ('v', "sh")];

/// 首道双拼的翘舌声母：v 为 zh，i 为 ch，e 为 sh。
pub const SHOUDAO_DIGRAPH_INITIALS: [(char, &str); 3] = [('v', "zh"), ('i', "ch"), ('e', "sh")];

/// 小鹤双拼。
pub const XIAOHE: Table = Table {
    digraph_initials: &DIGRAPH_INITIALS,
    finals: &[
        ('q', &["iu"]),
        ('w', &["ei"]),
        ('e', &["e"]),
        ('r', &["uan"]),
        ('t', &["ve", "ue"]),
        ('y', &["un"]),
        ('u', &["u"]),
        ('i', &["i"]),
        ('o', &["uo", "o"]),
        ('p', &["ie"]),
        ('a', &["a"]),
        ('s', &["iong", "ong"]),
        ('d', &["ai"]),
        ('f', &["en"]),
        ('g', &["eng"]),
        ('h', &["ang"]),
        ('j', &["an"]),
        ('k', &["ing", "uai"]),
        ('l', &["iang", "uang"]),
        ('z', &["ou"]),
        ('x', &["ia", "ua"]),
        ('c', &["ao"]),
        ('v', &["ui", "v"]),
        ('b', &["in"]),
        ('n', &["iao"]),
        ('m', &["ian"]),
    ],
    zero_initials: &[
        ("a", &["aa"]),
        ("ai", &["ai", "ad"]),
        ("an", &["an", "aj"]),
        ("ang", &["ah"]),
        ("ao", &["ao", "ac"]),
        ("e", &["ee"]),
        ("ei", &["ei", "ew"]),
        ("en", &["en", "ef"]),
        ("eng", &["eg"]),
        ("er", &["er"]),
        ("o", &["oo"]),
        ("ou", &["ou", "oz"]),
    ],
    semicolon: false,
};

/// 自然码。
pub const ZIRANMA: Table = Table {
    digraph_initials: &DIGRAPH_INITIALS,
    finals: &[
        ('q', &["iu"]),
        ('w', &["ia", "ua"]),
        ('e', &["e"]),
        ('r', &["uan"]),
        ('t', &["ve", "ue"]),
        ('y', &["uai", "ing"]),
        ('u', &["u"]),
        ('i', &["i"]),
        ('o', &["uo", "o"]),
        ('p', &["un"]),
        ('a', &["a"]),
        ('s', &["iong", "ong"]),
        ('d', &["iang", "uang"]),
        ('f', &["en"]),
        ('g', &["eng"]),
        ('h', &["ang"]),
        ('j', &["an"]),
        ('k', &["ao"]),
        ('l', &["ai"]),
        ('z', &["ei"]),
        ('x', &["ie"]),
        ('c', &["iao"]),
        ('v', &["ui", "v"]),
        ('b', &["ou"]),
        ('n', &["in"]),
        ('m', &["ian"]),
    ],
    zero_initials: &[
        ("a", &["aa"]),
        ("ai", &["ai", "al"]),
        ("an", &["an", "aj"]),
        ("ang", &["ah"]),
        ("ao", &["ao", "ak"]),
        ("e", &["ee"]),
        ("ei", &["ei", "ez"]),
        ("en", &["en", "ef"]),
        ("eng", &["eg"]),
        ("er", &["er"]),
        ("o", &["oo"]),
        ("ou", &["ou", "ob"]),
    ],
    semicolon: false,
};

/// 微软 / 搜狗共用的零声母写法：`o` 加韵母键，`a` / `e` 开头的也接受双写元音的写法。
const O_PREFIX_ZERO_INITIALS: &[(&str, &[&str])] = &[
    ("a", &["oa", "aa"]),
    ("ai", &["ol", "al"]),
    ("an", &["oj", "aj"]),
    ("ang", &["oh", "ah"]),
    ("ao", &["ok", "ak"]),
    ("e", &["oe", "ee"]),
    ("ei", &["oz", "ez"]),
    ("en", &["of", "ef"]),
    ("eng", &["og", "eg"]),
    ("er", &["or", "er"]),
    ("o", &["oo"]),
    ("ou", &["ob", "ou"]),
];

/// 微软双拼：ü 在 `y`，üe 在 `t`（`v` 也认），ing 在 `;`。
pub const MICROSOFT: Table = Table {
    digraph_initials: &DIGRAPH_INITIALS,
    finals: &[
        ('q', &["iu"]),
        ('w', &["ia", "ua"]),
        ('e', &["e"]),
        ('r', &["uan"]),
        ('t', &["ve", "ue"]),
        ('y', &["uai", "v"]),
        ('u', &["u"]),
        ('i', &["i"]),
        ('o', &["uo", "o"]),
        ('p', &["un"]),
        ('a', &["a"]),
        ('s', &["iong", "ong"]),
        ('d', &["iang", "uang"]),
        ('f', &["en"]),
        ('g', &["eng"]),
        ('h', &["ang"]),
        ('j', &["an"]),
        ('k', &["ao"]),
        ('l', &["ai"]),
        (';', &["ing"]),
        ('z', &["ei"]),
        ('x', &["ie"]),
        ('c', &["iao"]),
        ('v', &["ui", "ve", "ue"]),
        ('b', &["ou"]),
        ('n', &["in"]),
        ('m', &["ian"]),
    ],
    zero_initials: O_PREFIX_ZERO_INITIALS,
    semicolon: true,
};

/// 搜狗双拼：与微软只差 `v` 键不兼作 üe。
pub const SOGOU: Table = Table {
    digraph_initials: &DIGRAPH_INITIALS,
    finals: &[
        ('q', &["iu"]),
        ('w', &["ia", "ua"]),
        ('e', &["e"]),
        ('r', &["uan"]),
        ('t', &["ve", "ue"]),
        ('y', &["uai", "v"]),
        ('u', &["u"]),
        ('i', &["i"]),
        ('o', &["uo", "o"]),
        ('p', &["un"]),
        ('a', &["a"]),
        ('s', &["iong", "ong"]),
        ('d', &["iang", "uang"]),
        ('f', &["en"]),
        ('g', &["eng"]),
        ('h', &["ang"]),
        ('j', &["an"]),
        ('k', &["ao"]),
        ('l', &["ai"]),
        (';', &["ing"]),
        ('z', &["ei"]),
        ('x', &["ie"]),
        ('c', &["iao"]),
        ('v', &["ui"]),
        ('b', &["ou"]),
        ('n', &["in"]),
        ('m', &["ian"]),
    ],
    zero_initials: O_PREFIX_ZERO_INITIALS,
    semicolon: true,
};

/// 智能 ABC 的零声母写法：`o` 加韵母键。不像微软 / 搜狗还认双写元音——`aa` / `ee` 在 ABC 里是 zha / che。
const ABC_ZERO_INITIALS: &[(&str, &[&str])] = &[
    ("a", &["oa"]),
    ("ai", &["ol"]),
    ("an", &["oj"]),
    ("ang", &["oh"]),
    ("ao", &["ok"]),
    ("e", &["oe"]),
    ("ei", &["oq"]),
    ("en", &["of"]),
    ("eng", &["og"]),
    ("er", &["or"]),
    ("o", &["oo"]),
    ("ou", &["ob"]),
];

/// 智能 ABC：翘舌声母在 `a` / `e` / `v`（zh / ch / sh），零声母一律 `o` 前缀。
pub const ABC: Table = Table {
    digraph_initials: &ABC_DIGRAPH_INITIALS,
    finals: &[
        ('q', &["ei"]),
        ('w', &["ian"]),
        ('e', &["e"]),
        ('r', &["iu"]),
        ('t', &["iang", "uang"]),
        ('y', &["ing"]),
        ('u', &["u"]),
        ('i', &["i"]),
        ('o', &["uo", "o"]),
        ('p', &["uan"]),
        ('a', &["a"]),
        ('s', &["iong", "ong"]),
        ('d', &["ia", "ua"]),
        ('f', &["en"]),
        ('g', &["eng"]),
        ('h', &["ang"]),
        ('j', &["an"]),
        ('k', &["ao"]),
        ('l', &["ai"]),
        ('z', &["iao"]),
        ('x', &["ie"]),
        ('c', &["in", "uai"]),
        ('v', &["v"]),
        ('b', &["ou"]),
        ('n', &["un"]),
        ('m', &["ui", "ve", "ue"]),
    ],
    zero_initials: ABC_ZERO_INITIALS,
    semicolon: false,
};

/// 小浪双拼。
pub const XIAOLANG: Table = Table {
    digraph_initials: &XIAOLANG_DIGRAPH_INITIALS,
    finals: &[
        ('w', &["ei"]),
        ('e', &["e"]),
        ('r', &["ou"]),
        ('t', &["iu"]),
        ('y', &["un", "vn"]),
        ('u', &["u"]),
        ('i', &["i"]),
        ('o', &["uo", "o"]),
        ('p', &["ie"]),
        ('a', &["a"]),
        ('s', &["ao"]),
        ('d', &["ui", "in"]),
        ('f', &["ian", "ua"]),
        ('g', &["uan"]),
        ('h', &["ang"]),
        ('j', &["an", "iong"]),
        ('k', &["ai", "ia"]),
        ('l', &["ong"]),
        ('z', &["uang"]),
        ('x', &["v", "u"]),
        ('c', &["iao"]),
        ('v', &["uai", "ing"]),
        ('b', &["ve", "ue"]),
        ('n', &["eng"]),
        ('m', &["iang", "en"]),
    ],
    zero_initials: &[
        ("a", &["aa"]),
        ("ai", &["ai"]),
        ("an", &["an"]),
        ("ang", &["ah"]),
        ("ao", &["ao"]),
        ("e", &["uu"]),
        ("ei", &["ui"]),
        ("en", &["un"]),
        ("eng", &["un"]),
        ("er", &["ur"]),
        ("o", &["oo"]),
        ("ou", &["ou"]),
    ],
    semicolon: false,
};

/// 首道双拼的零声母写法：a / o 开头的双写首字母或照全拼敲（ang 是 `ay`），`e` 键让给了 sh，
/// 所以 e / ei / eng 改用 `u` 引导；`en` `er` 仍照全拼敲（sh 配不出 ian / ie，不撞）。
const SHOUDAO_ZERO_INITIALS: &[(&str, &[&str])] = &[
    ("a", &["aa"]),
    ("ai", &["ai"]),
    ("an", &["an"]),
    ("ang", &["ay"]),
    ("ao", &["ao"]),
    ("e", &["ue"]),
    ("ei", &["ui"]),
    ("en", &["en"]),
    ("eng", &["uf"]),
    ("er", &["er"]),
    ("o", &["oo"]),
    ("ou", &["ou"]),
];

/// 首道双拼：键位按作者公布的键位图（shoudaoshuangpin/shoudaoshouyouplus 的 `shoudao_layout.jpg`）。
/// ue（jue / que / xue / yue）在 `l`，üe（lve / nve）单独在 `b`。
pub const SHOUDAO: Table = Table {
    digraph_initials: &SHOUDAO_DIGRAPH_INITIALS,
    finals: &[
        ('q', &["iu"]),
        ('w', &["ua"]),
        ('e', &["e"]),
        ('r', &["ie"]),
        ('t', &["uan"]),
        ('y', &["ang"]),
        ('u', &["u"]),
        ('i', &["i"]),
        ('o', &["uo", "o"]),
        ('p', &["iao"]),
        ('a', &["a"]),
        ('s', &["ou"]),
        ('d', &["ao"]),
        ('f', &["eng"]),
        ('g', &["uai", "ing"]),
        ('h', &["ong", "iong"]),
        ('j', &["an"]),
        ('k', &["en", "ia"]),
        ('l', &["ai", "ue"]),
        ('z', &["un"]),
        ('x', &["iang", "uang"]),
        ('c', &["in"]),
        ('v', &["ui", "v"]),
        ('b', &["ve"]),
        ('n', &["ian"]),
        ('m', &["ei"]),
    ],
    zero_initials: SHOUDAO_ZERO_INITIALS,
    semicolon: false,
};

/// 拼音加加双拼的翘舌声母：v 为 zh，u 为 ch，i 为 sh。
pub const PYJJ_DIGRAPH_INITIALS: [(char, &str); 3] = [('v', "zh"), ('u', "ch"), ('i', "sh")];

/// 拼音加加双拼的零声母写法：`o` 加韵母键，以及 a / e 开头的双写首字母（`oa` / `aa`）。
const PYJJ_ZERO_INITIALS: &[(&str, &[&str])] = &[
    ("a", &["oa", "aa"]),
    ("ai", &["os", "as"]),
    ("an", &["of", "af"]),
    ("ang", &["og", "ag"]),
    ("ao", &["od", "ad"]),
    ("e", &["oe", "ee"]),
    ("ei", &["ow", "ew"]),
    ("en", &["or", "er"]),
    ("eng", &["ot", "et"]),
    ("er", &["oq", "eq"]),
    ("o", &["oo"]),
    ("ou", &["op", "ou"]),
];

/// 拼音加加双拼：键位按 Rime 的 `double_pinyin_pyjj.schema.yaml` 核对（中文维基「双拼」页的键位图与之一致）。
/// `q` 兼 ing 与 er（er 走零声母表），`x` 兼 üe 与 uai。
pub const PYJJ: Table = Table {
    digraph_initials: &PYJJ_DIGRAPH_INITIALS,
    finals: &[
        ('q', &["ing"]),
        ('w', &["ei"]),
        ('e', &["e"]),
        ('r', &["en"]),
        ('t', &["eng"]),
        ('y', &["iong", "ong"]),
        ('u', &["u"]),
        ('i', &["i"]),
        ('o', &["uo", "o"]),
        ('p', &["ou"]),
        ('a', &["a"]),
        ('s', &["ai"]),
        ('d', &["ao"]),
        ('f', &["an"]),
        ('g', &["ang"]),
        ('h', &["iang", "uang"]),
        ('j', &["ian"]),
        ('k', &["iao"]),
        ('l', &["in"]),
        ('z', &["un"]),
        ('x', &["ve", "ue", "uai"]),
        ('c', &["uan"]),
        ('v', &["ui", "v"]),
        ('b', &["ia", "ua"]),
        ('n', &["iu"]),
        ('m', &["ie"]),
    ],
    zero_initials: PYJJ_ZERO_INITIALS,
    semicolon: false,
};
