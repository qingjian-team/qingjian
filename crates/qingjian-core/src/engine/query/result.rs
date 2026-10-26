use crate::candidate::{Candidate, CandidateList};
use crate::correction::Correction;
use crate::parser::Segmentation;

/// 最优切分的音节用 `'` 连接，再接未切分尾部。
pub(crate) fn join_marked(segmentations: &[Segmentation], tail: &str) -> String {
    let mut text = segmentations
        .first()
        .map(|s| s.joined("'"))
        .unwrap_or_default();
    if !tail.is_empty() {
        if !text.is_empty() {
            text.push('\'');
        }
        text.push_str(tail);
    }
    text
}

/// 拼音行该按哪种切分画（`segmentations` 的下标）：与候选读音逐音节相同的最多，其次是候选音节以敲的开头（简拼、没打完）；
/// 一样多取靠前的。`ganga` 候选 尴尬 画成 `gan'ga`、刚啊 画成 `gang'a`；没有音节的候选（英文、快捷）按最优切分。
pub(crate) fn spelling_for(segmentations: &[Segmentation], candidate: Option<&Candidate>) -> usize {
    let Some(syllables) = candidate.map(|c| &c.syllables).filter(|s| !s.is_empty()) else {
        return 0;
    };
    let score = |segmentation: &Segmentation| -> usize {
        segmentation
            .syllables
            .iter()
            .zip(syllables)
            .map(
                |(typed, read)| match read.strip_prefix(typed.text.as_str()) {
                    Some("") => 2,
                    Some(_) => 1,
                    None => 0,
                },
            )
            .sum()
    };
    let mut best = (0, 0);
    for (index, segmentation) in segmentations.iter().enumerate() {
        let score = score(segmentation);
        if score > best.1 {
            best = (index, score);
        }
    }
    best.0
}

/// 与 [`join_marked`] 相同的分段，但用原样大小写的输入（`Cpan`）：切分是按小写算的，
/// 大小写只影响显示，逐段按同样的字节长度取回原样文本。
pub(crate) fn join_marked_typed(typed: &str, segmentations: &[Segmentation], tail: &str) -> String {
    let mut text = String::new();
    let mut offset = 0;
    if let Some(first) = segmentations.first() {
        for (index, syllable) in first.syllables.iter().enumerate() {
            if index > 0 {
                text.push('\'');
            }
            let end = (offset + syllable.text.len()).min(typed.len());
            text.push_str(&typed[offset..end]);
            offset = end;
        }
    }
    if !tail.is_empty() {
        if !text.is_empty() {
            text.push('\'');
        }
        text.push_str(&typed[offset.min(typed.len())..]);
    }
    text
}

use crate::engine::timings::Timings;
use crate::engine::{AuxSegment, MarkedKind, MarkedSegment};

/// 不带译文的候选查询结果。
#[derive(Debug, Clone, Default)]
pub struct Query {
    /// 参与候选生成的所有切分，索引 0 为首选切分。
    pub segmentations: Vec<Segmentation>,

    /// 拼音行按 `segmentations` 里的哪一种画：缺省对着首选候选挑（见 [`Self::follow`]）。
    pub spelled: usize,

    /// 排好序的候选，`translation` 均为 `None`。
    pub candidates: CandidateList,

    /// 输入末尾无法切分的字母（如 `kaifv` 的 `v`），不参与本次候选，留给后续输入。
    pub tail: String,

    /// 查询时缓冲区里的原始文本。
    pub text: String,

    /// 查询时的光标位置（`text` 的字节下标）。
    pub cursor: usize,

    /// 光标停在中间时，作用域之后剩下的拼音的显示形式（已按音节用 `'` 连好）；候选不管它，只画出来。
    pub rest: String,

    /// 各阶段耗时。
    pub timings: Timings,

    /// 生效的拼写纠正：`segmentations` 与候选都来自纠正后的拼音，`text` 仍是用户敲的。
    pub correction: Option<Correction>,

    /// 双拼 / 注音开着：`segmentations` 是解出来的拼音，`text` 是敲的键，两者长度对不上。
    pub decoded_keys: bool,

    /// 双拼模式下是否在 preedit（应用输入框内 marked text）保留原始输入按键。
    pub shuangpin_raw_preedit: bool,

    /// 开启双拼或注音时的显示字串（如 "ㄅㄨˋ"）。如果有此值，preedit 就优先显示它，而不是拼音。
    pub typed_display: Option<String>,

    /// 中文模式下 Shift 敲了大写时敲的原样（`Cpan`）：匹配按小写算，拼音行按它的大小写画。
    pub typed_case: Option<String>,

    /// 九宫格的拼音列：第一个没定下来的位置上的全部写法，最可能的在前；壳点了其中一个就调 `Engine::pin_nine_key`。
    pub pinyin_choices: Vec<String>,

    /// 辅码态：触发键与码段（码段可为空——刚触发）。`Some` 时 preedit 在拼音段之后多出
    /// [触发键 `Typed`][码段 `AuxCode`] 两段，候选也已经按码段筛过。
    pub aux: Option<AuxSegment>,

    /// 同一段拼音里前面选好、还没上屏的词（见 [`crate::Engine::take_held`]），显示在拼音前面。
    pub held: String,
}

impl Query {
    /// 无法解析为拼音但精确匹配自定义短语时，保留原始输入和光标。
    pub(super) fn custom_only(
        text: &str,
        cursor: usize,
        decoded_keys: bool,
        shuangpin_raw_preedit: bool,
        scope: &str,
        rest: String,
    ) -> Self {
        Self {
            text: text.to_owned(),
            cursor,
            decoded_keys,
            shuangpin_raw_preedit,
            tail: scope.to_owned(),
            rest,
            ..Self::default()
        }
    }

    /// 拼音行改按 `candidate` 的读音切分（壳换了高亮候选时调）：`ganga` 高亮 尴尬 时是 `gan'ga`。
    pub fn follow(&mut self, candidate: Option<&Candidate>) {
        self.spelled = self.spelling_of(candidate);
    }

    /// [`Self::follow`] 会换成的切分下标，不改自己：与 `spelled` 相同就不必重画。
    pub fn spelling_of(&self, candidate: Option<&Candidate>) -> usize {
        spelling_for(&self.segmentations, candidate)
    }

    /// 给 marked text（应用输入框未上屏文本）用的显示形式：`spelled` 那种切分的音节用 `'` 连接，再接未切分尾部，
    /// 辅码态接上触发键与码段，光标后的剩余拼音跟在最后。
    /// `kaifa` → `kai'fa`，`kf` → `k'f`，`ni|hao` → `ni'hao`，`nihao;rb` → `ni'hao;rb`。
    /// 双拼模式且 `shuangpin_raw_preedit` 开启时，返回原始按键（如 `kdfa`）。
    pub fn marked_text(&self) -> String {
        if self.shuangpin_raw_preedit {
            return format!("{}{}", self.held, self.text);
        }
        self.marked_segments()
            .iter()
            .map(|s| s.text.as_str())
            .collect()
    }

    /// [`Self::marked_text`] 的分段形式：前面选好还没上屏的词与敲的拼音各一段（`Typed`），
    /// 光标后剩下的拼音连同前面的 `'` 一段（`Rest`）。
    /// 壳按段画样式；[`Self::segments_cursor`] 的位置按各段拼接后的字符数算。
    pub fn marked_segments(&self) -> Vec<MarkedSegment> {
        let mut segments = Vec::with_capacity(3);
        if !self.held.is_empty() {
            segments.push(MarkedSegment::new(self.held.clone(), MarkedKind::Typed));
        }
        segments.extend(self.pinyin_segments(None));
        segments
    }

    /// 候选窗口顶部的拼音行与光标。带调拼音开着、高亮候选读得出声调（`toned`，见
    /// [`crate::Engine::toned_syllables`]）时，敲的那段换成候选的带调读音，候选没覆盖到的音节照切分接在后面
    /// （`nihao` 高亮 你好 是 `nǐ'hǎo`、高亮 你 是 `nǐ'hao`，简拼 `nh` 高亮 你好 也是 `nǐ'hǎo`）；纠错生效时不换。
    /// 应用输入框里的 marked text 不受影响，仍是 [`Self::marked_text`]。
    pub fn window_segments(&self, toned: Option<&[String]>) -> (Vec<MarkedSegment>, usize) {
        let spelled = self.segmentations.get(self.spelled);
        // 读音对不上敲的（模糊音、敲错变体、云端推荐）不换：拼音行仍是用户敲的
        let fits = |toned: &[String]| {
            spelled.is_some_and(|spelled| {
                spelled.syllables.iter().zip(toned).all(|(typed, read)| {
                    qingjian_dictionary::strip_marks(read).starts_with(typed.text.as_str())
                })
            })
        };
        let Some(toned) = toned.filter(|t| !t.is_empty() && self.correction.is_none() && fits(t))
        else {
            return (self.marked_segments(), self.segments_cursor());
        };
        let mut parts: Vec<&str> = toned.iter().map(String::as_str).collect();
        if let Some(spelled) = spelled {
            parts.extend(
                spelled
                    .syllables
                    .iter()
                    .skip(toned.len())
                    .map(|s| s.text.as_str()),
            );
        }
        if !self.tail.is_empty() {
            parts.push(&self.tail);
        }
        let mut segments = Vec::with_capacity(4);
        if !self.held.is_empty() {
            segments.push(MarkedSegment::new(self.held.clone(), MarkedKind::Typed));
        }
        segments.extend(self.pinyin_segments(Some(parts.join("'"))));
        // 光标总在作用域末尾：作用域就是光标前的那段，后面只剩 `Rest`
        let cursor = segments
            .iter()
            .filter(|s| s.kind != MarkedKind::Rest)
            .map(|s| s.text.chars().count())
            .sum();
        (segments, cursor)
    }

    /// 拼音部分（不含前面留着的词）的分段；`typed` 给了就用它当敲的那段（[`Self::window_segments`]）。
    fn pinyin_segments(&self, typed: Option<String>) -> Vec<MarkedSegment> {
        let mut segments = match &self.correction {
            Some(correction) => correction.marked_segments(),
            None => Vec::with_capacity(2),
        };
        if self.correction.is_none() {
            let typed = if let Some(typed) = typed {
                typed
            } else if let Some(display) = &self.typed_display {
                display.clone()
            } else {
                let spelled = self.segmentations.get(self.spelled..).unwrap_or_default();
                match &self.typed_case {
                    Some(typed) => join_marked_typed(typed, spelled, &self.tail),
                    None => join_marked(spelled, &self.tail),
                }
            };
            if !typed.is_empty() {
                segments.push(MarkedSegment::new(typed, MarkedKind::Typed));
            }
        }
        if let Some(aux) = &self.aux {
            // 触发键按 Typed 画（不突出），码段是新段类型；码段为空时只多这一段触发键
            segments.push(MarkedSegment::new(
                aux.trigger.to_string(),
                MarkedKind::Typed,
            ));
            if !aux.code.is_empty() {
                segments.push(MarkedSegment::new(aux.code.clone(), MarkedKind::AuxCode));
            }
        }
        if !self.rest.is_empty() {
            let rest = if segments.is_empty() {
                self.rest.clone()
            } else {
                format!("'{}", self.rest)
            };
            segments.push(MarkedSegment::new(rest, MarkedKind::Rest));
        }
        segments
    }

    /// 光标在 [`Self::marked_segments`] 拼接文本里的字符下标（候选窗口顶部拼音行使用）。
    pub fn segments_cursor(&self) -> usize {
        self.held.chars().count() + self.pinyin_cursor()
    }

    /// 光标在拼音部分（[`Self::pinyin_segments`]）里的字符下标。
    fn pinyin_cursor(&self) -> usize {
        // 纠错生效、解码时显示串与敲的不一样长，作用域又总在光标前：光标就在敲的部分末尾
        // （光标在开头时作用域是整段，光标仍在开头）
        if self.decoded_keys && self.cursor == 0 {
            return 0;
        }
        // 辅码态光标也总在末尾：触发键与码段都拼在最后
        if self.correction.is_some() || self.decoded_keys || self.aux.is_some() {
            return self
                .marked_segments()
                .iter()
                .filter(|s| s.kind != MarkedKind::Rest)
                .map(|s| s.text.chars().count())
                .sum();
        }
        let letters_before = self.text[..self.cursor.min(self.text.len())]
            .chars()
            .filter(|c| *c != '\'')
            .count();
        let after_apostrophe = self.text[..self.cursor.min(self.text.len())].ends_with('\'');
        let segments_text: String = self
            .marked_segments()
            .iter()
            .map(|s| s.text.as_str())
            .collect();
        let chars: Vec<char> = segments_text.chars().collect();
        let mut seen = 0;
        let mut position = 0;
        while position < chars.len() && seen < letters_before {
            if chars[position] != '\'' {
                seen += 1;
            }
            position += 1;
        }
        if after_apostrophe && chars.get(position) == Some(&'\'') {
            position += 1;
        }
        position
    }

    /// 光标在 [`Self::marked_text`] 里的字符下标（给平台层传给宿主应用输入框用的，所以按字符算，不是字节）。
    pub fn marked_cursor(&self) -> usize {
        if self.shuangpin_raw_preedit {
            return self.held.chars().count()
                + self.text[..self.cursor.min(self.text.len())]
                    .chars()
                    .count();
        }
        self.segments_cursor()
    }
}
