//! 罗马音 → 平假名：常见日文输入法的拼法（训令 / 平文都认），最长匹配。

/// 罗马音与平假名，按键长从长到短查。
const TABLE: &[(&str, &str)] = &[
    ("ltsu", "っ"),
    ("xtsu", "っ"),
    ("kya", "きゃ"),
    ("kyu", "きゅ"),
    ("kyo", "きょ"),
    ("gya", "ぎゃ"),
    ("gyu", "ぎゅ"),
    ("gyo", "ぎょ"),
    ("sha", "しゃ"),
    ("shi", "し"),
    ("shu", "しゅ"),
    ("she", "しぇ"),
    ("sho", "しょ"),
    ("sya", "しゃ"),
    ("syu", "しゅ"),
    ("syo", "しょ"),
    ("zya", "じゃ"),
    ("zyu", "じゅ"),
    ("zyo", "じょ"),
    ("jya", "じゃ"),
    ("jyu", "じゅ"),
    ("jyo", "じょ"),
    ("cha", "ちゃ"),
    ("chi", "ち"),
    ("chu", "ちゅ"),
    ("che", "ちぇ"),
    ("cho", "ちょ"),
    ("tya", "ちゃ"),
    ("tyu", "ちゅ"),
    ("tyo", "ちょ"),
    ("dya", "ぢゃ"),
    ("dyu", "ぢゅ"),
    ("dyo", "ぢょ"),
    ("tsu", "つ"),
    ("thi", "てぃ"),
    ("dhi", "でぃ"),
    ("nya", "にゃ"),
    ("nyu", "にゅ"),
    ("nyo", "にょ"),
    ("hya", "ひゃ"),
    ("hyu", "ひゅ"),
    ("hyo", "ひょ"),
    ("bya", "びゃ"),
    ("byu", "びゅ"),
    ("byo", "びょ"),
    ("pya", "ぴゃ"),
    ("pyu", "ぴゅ"),
    ("pyo", "ぴょ"),
    ("mya", "みゃ"),
    ("myu", "みゅ"),
    ("myo", "みょ"),
    ("rya", "りゃ"),
    ("ryu", "りゅ"),
    ("ryo", "りょ"),
    ("ltu", "っ"),
    ("xtu", "っ"),
    ("lya", "ゃ"),
    ("lyu", "ゅ"),
    ("lyo", "ょ"),
    ("xya", "ゃ"),
    ("xyu", "ゅ"),
    ("xyo", "ょ"),
    ("ka", "か"),
    ("ki", "き"),
    ("ku", "く"),
    ("ke", "け"),
    ("ko", "こ"),
    ("ga", "が"),
    ("gi", "ぎ"),
    ("gu", "ぐ"),
    ("ge", "げ"),
    ("go", "ご"),
    ("sa", "さ"),
    ("si", "し"),
    ("su", "す"),
    ("se", "せ"),
    ("so", "そ"),
    ("za", "ざ"),
    ("zi", "じ"),
    ("zu", "ず"),
    ("ze", "ぜ"),
    ("zo", "ぞ"),
    ("ja", "じゃ"),
    ("ji", "じ"),
    ("ju", "じゅ"),
    ("je", "じぇ"),
    ("jo", "じょ"),
    ("ta", "た"),
    ("ti", "ち"),
    ("tu", "つ"),
    ("te", "て"),
    ("to", "と"),
    ("da", "だ"),
    ("di", "ぢ"),
    ("du", "づ"),
    ("de", "で"),
    ("do", "ど"),
    ("na", "な"),
    ("ni", "に"),
    ("nu", "ぬ"),
    ("ne", "ね"),
    ("no", "の"),
    ("ha", "は"),
    ("hi", "ひ"),
    ("hu", "ふ"),
    ("he", "へ"),
    ("ho", "ほ"),
    ("fa", "ふぁ"),
    ("fi", "ふぃ"),
    ("fu", "ふ"),
    ("fe", "ふぇ"),
    ("fo", "ふぉ"),
    ("ba", "ば"),
    ("bi", "び"),
    ("bu", "ぶ"),
    ("be", "べ"),
    ("bo", "ぼ"),
    ("pa", "ぱ"),
    ("pi", "ぴ"),
    ("pu", "ぷ"),
    ("pe", "ぺ"),
    ("po", "ぽ"),
    ("ma", "ま"),
    ("mi", "み"),
    ("mu", "む"),
    ("me", "め"),
    ("mo", "も"),
    ("ya", "や"),
    ("yu", "ゆ"),
    ("yo", "よ"),
    ("ra", "ら"),
    ("ri", "り"),
    ("ru", "る"),
    ("re", "れ"),
    ("ro", "ろ"),
    ("la", "ぁ"),
    ("li", "ぃ"),
    ("lu", "ぅ"),
    ("le", "ぇ"),
    ("lo", "ぉ"),
    ("xa", "ぁ"),
    ("xi", "ぃ"),
    ("xu", "ぅ"),
    ("xe", "ぇ"),
    ("xo", "ぉ"),
    ("wa", "わ"),
    ("wo", "を"),
    ("va", "ゔぁ"),
    ("vi", "ゔぃ"),
    ("vu", "ゔ"),
    ("ve", "ゔぇ"),
    ("vo", "ゔぉ"),
    ("nn", "ん"),
    ("n'", "ん"),
    ("a", "あ"),
    ("i", "い"),
    ("u", "う"),
    ("e", "え"),
    ("o", "お"),
    ("-", "ー"),
];

const MAX_KEY: usize = 4;

fn lookup(key: &str) -> Option<&'static str> {
    TABLE.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

/// 还能不能再敲成表里的某一条（`ky` → `kya`），能的话先不转、留作罗马音。
fn is_prefix(key: &str) -> bool {
    TABLE
        .iter()
        .any(|(k, _)| k.len() > key.len() && k.starts_with(key))
}

fn is_consonant(c: char) -> bool {
    c.is_ascii_lowercase() && !matches!(c, 'a' | 'i' | 'u' | 'e' | 'o')
}

/// 把罗马音 `raw` 转成平假名：返回（转好的假名，末尾还没拼完的罗马音）。
/// 同一辅音连打转成「っ」；`n` 后面接 `y` 和元音以外的字母转成「ん」。转不了的字符原样留下。
pub fn to_hiragana(raw: &str) -> (String, String) {
    let chars: Vec<char> = raw.chars().map(|c| c.to_ascii_lowercase()).collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let rest = &chars[i..];
        let longest = (1..=MAX_KEY.min(rest.len())).rev().find_map(|len| {
            lookup(&rest[..len].iter().collect::<String>()).map(|kana| (len, kana))
        });
        if let Some((len, kana)) = longest {
            out.push_str(kana);
            i += len;
            continue;
        }
        let (c, next) = (rest[0], rest.get(1).copied());
        if is_consonant(c) && c != 'n' && next == Some(c) {
            out.push('っ');
            i += 1;
        } else if c == 'n' && next.is_some_and(|n| is_consonant(n) && n != 'y') {
            out.push('ん');
            i += 1;
        } else if is_prefix(&rest.iter().collect::<String>()) {
            return (out, rest.iter().collect());
        } else {
            out.push(c);
            i += 1;
        }
    }
    (out, String::new())
}

/// 平假名换成片假名（「ー」与非假名原样）。
pub fn to_katakana(hiragana: &str) -> String {
    hiragana
        .chars()
        .map(|c| match c {
            '\u{3041}'..='\u{3096}' => char::from_u32(c as u32 + 0x60).unwrap_or(c),
            _ => c,
        })
        .collect()
}
