//! 候选排序。
//!
//! 词级排序规则：
//! 1. 音节数与输入完全一致的词优先（`kaifa` → 开发 排在 开发者 前）
//! 2. 覆盖输入字母多者优先（`kaif` → 开发者 排在 开 前）
//! 3. 切分里非末尾的简拼音节少者优先（`kaifa` 按 `kai fa` 读的 开放 排在按 `kai f a` 读的 开放啊 前）
//! 4. 最后一个音节完整匹配优先（`kaifa` → 开发 排在 开放 前）
//! 5. 注音敲了声调时，声调代价小者优先（ㄇㄚ 一声 → 妈 排在 吗 前；表与规则见 `zhuyin::tone`）
//! 6. 同一输入串下用户选过的次数（Learner 的 `choice_weight`：`mgs` 选过 美国式，下次 `mgs` 它就是首选）
//! 7. 上下文得分：语言模型给的 `log P(词 | 上一个上屏的词)`（个人 bigram 插值，模型不认识的按词库词频兜底并扣分，
//!    见 `sentence::transition_log_prob`）加用户选择次数的加分（[`weight_bonus`]，对数且封顶），模糊音命中扣 ln 2、
//!    敲错变体命中扣那类敲错的代价（`correction::TypoKind::cost`，个人敲错表打折）。
//!    这样 `ba` 在「做了」后面出 吧、句首出 把；纯词频排序两处都只能出同一个
//! 8. 敲的原音节优先，词长短者优先，最后按字符串稳定排序保证结果可复现
//!
//! 词库静态词频只用于预选（命中太多时先按词频砍到够排的量）与兜底。
//!
//! 排完再由 [`arrange`] 按类别重排一次：首音节的单字不被猜出来的长词、拆开音节的读法与几十个同音整词挤到几页之后；
//! 整句、emoji 插完、选词模型重排后 [`keep_single_char_on_first_page`] 再保证第一个单字在第一页。

mod first_page;
mod scored;
mod single_chars;

use std::collections::HashSet;

pub use first_page::{FIRST_SINGLE_CHAR_BY, keep_single_char_on_first_page};
pub use scored::{PreselectKey, Scored, SortKey};
pub use single_chars::keep_only_single_chars;

/// 用户选择次数的加分系数：加分 = 系数 × ln(1 + min(次数, [`WEIGHT_CAP`]))。
/// 这个加分只负责「同音词里偏向用户常选的那个」，所以既取对数又封顶（最多约 1.5 分，e^1.5 ≈ 4.5 倍），
/// 让上下文（bigram）仍能压过它；不封顶时选过 173 次的 的 会带着 5 分加分把任何含 的 的拆分路径抬到整词之上
/// （`haode` 出 号的 而不是 好的）。用户用得多的词另有个人 bigram 里的一元项撑腰，不靠这里。
pub const WEIGHT_BONUS: f64 = 0.5;

/// 加分里计入的选择次数上限。
pub const WEIGHT_CAP: u32 = 20;

/// 模糊音命中扣的分（词频减半）。敲错变体的代价见 `correction::TypoKind`。
pub const FUZZY_PENALTY: f64 = std::f64::consts::LN_2;

/// 预测补全（以输入为前缀、多出几个没敲的音节的长词：`shishan` → 时尚潮流）排在前缀词之前的最多条数，其余排到前缀词后面。
/// 整段没有词时第二页起就该是逐字选的单字（`shishan` 要 屎山，词库里没有），而不是几十个猜出来的长词。
pub const COMPLETIONS_BEFORE_PREFIXES: usize = 3;

/// 首音节单字之前最多几个词（整词、猜测、前缀词合计，主词库的优先）：其余的都排到单字后面。
/// 候选按句子、词、单字分段，词不多占位置：`ceshi` 给 测试 侧石 侧视 几个就是 测 侧 册，词库里没有想要的词时
/// 第一二页就能逐字选（#769、#812）；长句的前缀词绝大多数凑不成话（`niyouhualema` 的 你邮、泥涌、尼永）。
pub const WORDS_BEFORE_SINGLE_CHARS: usize = 6;

/// 多音节输入时首音节的单字单独留这么多个（不和词一起预选）：简拼 `ss` 读得出几万个两字词，
/// 合在一起按覆盖字母数预选会把单字全砍掉；几页够逐字选了。
pub const FIRST_SYLLABLE_CHARS_KEPT: usize = 100;

/// 首音节其余单字之前至多排几个词（含单字前的 [`WORDS_BEFORE_SINGLE_CHARS`] 个），再多的排到全部单字之后。
/// 简拼 `ss` 读得出几千个两字词，不封顶时单字要翻五十多页。按输入日志定：冻结日志与一份本机日志共一万四千条
/// 多音节选词，没有一条选在第 30 位以后（最靠后第 29 位，99% 在前 4 位）。
pub const WORDS_BEFORE_ALL_SINGLE_CHARS: usize = 30;

/// 有整词时拆开音节的读法排在首音节的几个单字之后（一页）。
pub const SINGLE_CHARS_BEFORE_SPLIT: usize = 9;

/// 有整词时拆开音节的读法在首音节其余单字之前至多留几条，其余排到全部单字之后。拆开的音节按声母通配，
/// `nihao` 读成 `ni h… ao` 能捞出 你会 你回 霓虹 几十条读音对不上的词，不限量就把单字挤到几页之后（#854）；
/// 留下的只给读音与敲的只差一个字母的（`nishui` 想打 你说、`duizhao` 想打 对着），当敲错音的兜底。
pub const SPLIT_READINGS_BEFORE_SINGLES: usize = 3;

/// 一串键能整段读出的用户词（自造词、接受过的云端词）至多这么多个当作在这串键下选过，不占
/// [`WORDS_BEFORE_SINGLE_CHARS`] 的名额、排在没选过的词前面（按用户选过的次数挑）；其余的照常排。
/// 简拼（`ss`）能对上十几个自造词，全提上来又会把单字挤到几页之后。
pub const USER_WORDS_BEFORE_SINGLES: usize = 5;

/// 模糊音命中的单字排在原样读音的头几个单字之后（一页）。模糊音只扣 ln 2，单字之间词频差得远，
/// 光靠扣分 `zi` 开 z-zh 首选会是 只 而不是 子；不排在这里则沉到原样读音的几百个生僻字之后。
pub const SINGLE_CHARS_BEFORE_FUZZY: usize = 9;

/// 用户选择次数换算成得分加成，词级排序与整句路径共用，见 [`WEIGHT_BONUS`]。
pub fn weight_bonus(count: u32) -> f64 {
    WEIGHT_BONUS * (1.0 + f64::from(count.min(WEIGHT_CAP))).ln()
}

/// 排序并按词文本去重（同一个词可能被多种切分命中，保留得分最高的一条），最多留 `limit` 条。
/// `context` 给每条命中算（同输入串下的选择次数, 上下文 log 概率），只对预选后剩下的那些调用。
///
/// 排序键先算好再排：单字母简拼能命中两万条，比较器里每次数字符数会让排序占掉几十毫秒；去重也只做到够数为止。
pub fn rank(
    items: &mut Vec<Scored<'_>>,
    limit: usize,
    context: impl Fn(&Scored<'_>) -> (u32, f64),
) {
    // 远超上限时先按结构键 + 词频线性选出前面一段：同一个词会被多种切分命中，多选一倍留给去重（结果仍可能略少于上限，无妨）
    let preselect = limit.saturating_mul(2);
    if items.len() > preselect.saturating_mul(2) {
        let mut keyed: Vec<(PreselectKey, Scored<'_>)> = items
            .drain(..)
            .map(|item| (item.preselect_key(), item))
            .collect();
        keyed.select_nth_unstable_by(preselect, |a, b| b.0.cmp(&a.0));
        keyed.truncate(preselect);
        items.extend(keyed.into_iter().map(|(_, item)| item));
    }
    let mut keyed: Vec<(SortKey<'_>, Scored<'_>)> = items
        .drain(..)
        .map(|item| {
            let (choice, log_prob) = context(&item);
            let score = log_prob + weight_bonus(item.weight) - item.penalty;
            (item.key(choice, score), item)
        })
        .collect();
    keyed.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    let mut seen: HashSet<&str> = HashSet::with_capacity(limit.min(keyed.len()));
    items.extend(
        keyed
            .into_iter()
            .map(|(_, item)| item)
            .filter(|item| seen.insert(item.hit.text))
            .take(limit),
    );
}

/// 模糊音命中的单字等原样读音的头 [`SINGLE_CHARS_BEFORE_FUZZY`] 个单字之后再出，其余顺序不变；选过的（`chosen`）不等。
fn defer_altered_chars(items: &mut Vec<Scored<'_>>, chosen: impl Fn(&Scored<'_>) -> bool) {
    let mut typed_chars = 0;
    let mut waiting = Vec::new();
    for item in std::mem::take(items) {
        let single = item.hit.text.chars().count() == 1;
        if typed_chars < SINGLE_CHARS_BEFORE_FUZZY && single && item.altered() && !chosen(&item) {
            waiting.push(item);
            continue;
        }
        items.push(item);
        if single && !item.altered() {
            typed_chars += 1;
            if typed_chars == SINGLE_CHARS_BEFORE_FUZZY {
                items.append(&mut waiting);
            }
        }
    }
    items.append(&mut waiting);
}

/// 排好序之后按类别重排一次，各类内部保持 [`rank`] 的顺序。输入不止一个音节时：
/// 1. 词，至多 [`WORDS_BEFORE_SINGLE_CHARS`] 个：先是盖住整段、读音就是敲的那几个音节的词（主词库的在前），再是至多
///    [`COMPLETIONS_BEFORE_PREFIXES`] 条猜测（比敲的多出音节的长词 `shishan` → 时尚潮流、把打完的末音节读成更长音节的词：
///    `shan` 读成 `shang` 的 事上；只在 `clean`，即输入有一种全是完整音节的切法时算猜测，末音节还没打完时 `jixub`
///    补全正是用户要往下打的，算作整词），最后是只盖住开头一段的主词库前缀词；
/// 2. 首音节单字的头 [`SINGLE_CHARS_BEFORE_SPLIT`] 个；
/// 3. 其余盖住整段的词（`xinjin` 的 新进 这类排在第 7 个以后的，不沉到几百个生僻单字后面）；
/// 4. 拆开音节的读法里读音与敲的只差一个字母的（`near_typed`），至多 [`SPLIT_READINGS_BEFORE_SINGLES`] 条，其余排在第 5 类全部之后——只在有词的读音正好是敲的音节时这样放；一个都没有时（`wodei` 的 我的、`cankai` 的 参考、`mingtia` 的 明天）
///    它们多半是手误或没打完，正是用户要的，按词算；
/// 5. 其余的词（连同第 1 类至多 [`WORDS_BEFORE_ALL_SINGLE_CHARS`] 个）、其余首音节单字、再多的词、其余猜测、补充词库的前缀词。
///
/// `whole` 是整段输入的字母数；`complete` 是其中完整音节的字母数（末尾没打完的音节不算：`woganj` 是 5），盖住它的词（我敢）
/// 也算整词；`first` 是最好那种切法首音节的字母数（`daoshih` 是 `dao` 的 3，不是简拼 `d` 的 1）。
/// 这个输入串下选过的（`chosen`）留在第 1 类里 [`rank`] 给的位置，不算进名额，也不被挤到单字后面。
/// 输入只有一个音节时单字就是整词，顺序与 [`rank`] 一样，只把多出来的猜测排到后面、模糊音命中的单字排到原样读音的
/// 头 [`SINGLE_CHARS_BEFORE_FUZZY`] 个单字之后（多个音节时模糊音命中的词照得分排，`lihao` 开 n-l 首选仍是 你好）。
pub fn arrange(
    items: &mut Vec<Scored<'_>>,
    whole: usize,
    complete: usize,
    first: usize,
    clean: bool,
    chosen: impl Fn(&Scored<'_>) -> bool,
    near_typed: impl Fn(&Scored<'_>) -> bool,
) {
    let worded = items
        .iter()
        .any(|item| !item.split && item.coverage >= whole);
    let several = whole > first;
    if !several {
        defer_altered_chars(items, &chosen);
    }
    let single =
        |item: &Scored<'_>| several && item.coverage == first && item.hit.text.chars().count() == 1;
    // 选过的（第二项为真）照 `rank` 的位置留在整词里：不提到最前面，也不被名额挤到单字后面
    let mut words = Vec::with_capacity(items.len());
    let mut guesses = Vec::new();
    let mut prefixes = Vec::new();
    let mut singles = Vec::new();
    let mut split = Vec::new();
    for item in items.drain(..) {
        if chosen(&item) {
            words.push((item, true));
        } else if item.split && worded {
            split.push(item);
        } else if single(&item) {
            singles.push(item);
        } else if item.coverage < whole && item.coverage < complete {
            prefixes.push((item, false));
        } else if item.coverage < whole {
            words.push((item, false));
        } else if clean && (!item.hit.exact || !item.full_last) {
            guesses.push((item, false));
        } else {
            words.push((item, false));
        }
    }
    let late = guesses.split_off(guesses.len().min(COMPLETIONS_BEFORE_PREFIXES));
    // 补充词库：盖住整段的（rizhi 的 日职、日制）跟在主词库的整词后面照样占名额，前缀词多半凑不成话（你邮、泥涌），排最后
    let movable = |(item, kept): &(Scored<'_>, bool)| several && !kept && item.hit.secondary;
    let (whole_secondary, mut words): (Vec<_>, Vec<_>) = words.into_iter().partition(movable);
    let (secondary, prefixes): (Vec<_>, Vec<_>) = prefixes.into_iter().partition(movable);
    words.extend(whole_secondary);
    words.extend(guesses);
    words.extend(prefixes);
    let mut budget = if several {
        WORDS_BEFORE_SINGLE_CHARS
    } else {
        usize::MAX
    };
    let (front, rest): (Vec<_>, Vec<_>) = words.into_iter().partition(|(_, kept)| {
        if *kept {
            return true;
        }
        let fits = budget > 0;
        budget = budget.saturating_sub(1);
        fits
    });
    let (rest_whole, rest): (Vec<_>, Vec<_>) = rest
        .into_iter()
        .partition(|(item, _)| item.coverage >= complete);
    let more_singles = singles.split_off(singles.len().min(SINGLE_CHARS_BEFORE_SPLIT));
    let (mut split, far_split): (Vec<_>, Vec<_>) = split.into_iter().partition(&near_typed);
    let more_split = split.split_off(split.len().min(SPLIT_READINGS_BEFORE_SINGLES));
    let room = WORDS_BEFORE_ALL_SINGLE_CHARS.saturating_sub(front.len());
    let mut later: Vec<Scored<'_>> = rest_whole
        .into_iter()
        .map(|(item, _)| item)
        .chain(split)
        .chain(rest.into_iter().map(|(item, _)| item))
        .collect();
    let overflow = later.split_off(later.len().min(room));
    items.extend(front.into_iter().map(|(item, _)| item));
    items.extend(singles);
    items.extend(later);
    items.extend(more_singles);
    items.extend(overflow);
    items.extend(late.into_iter().map(|(item, _)| item));
    items.extend(secondary.into_iter().map(|(item, _)| item));
    items.extend(more_split);
    items.extend(far_split);
}

#[cfg(test)]
mod tests;
