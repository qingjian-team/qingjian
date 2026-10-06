//! 词图读不通整段输入时的兜底候选：让模型直接按按键生成整句。

use crate::candidate::Candidate;
use crate::candidate::CandidateKind;
use crate::engine::Engine;
use crate::engine::MIN_GENERATED_LETTERS;
use crate::parser::Segmentation;
use crate::sentence;

impl Engine {
    /// 模型直接按整段按键生成的整句候选，最好的在前；不该生成或生成不出来时是空的。
    ///
    /// 只在**词图读不通整段**时才走，三种读不通：
    ///
    /// - 最优切分里有读不完整的音节（`yongdockerbushuhenfangbian` → `yong d… o c… ke r… bu shu…`）；
    /// - 切分压根没覆盖到末尾（`woyongvscodexiedaima` 切到 `wo yong` 就停了，`v` 起不了音节）；
    /// - 拼写纠错生效了（`corrected`）——`womaileyigeiphone` 里的 `phone` 被当成敲错的 `paone`，
    ///   纠完切分就「干净」了，前两条都看不出来。纠错本来判的就是「这段不像拼音」，与这里是同一个问题。
    ///
    /// 三种都说明整段里有一截不是拼音：中英混输的英文段，或者打得太离谱。这时词图只能把那一截
    /// 硬读成拼音、丢掉后半句、或者纠成别的字，出来的必然是废话（用的哦乘客仍不熟很方便 / 我用 /
    /// 我买了一给跑呢），生成的几十毫秒花得值。
    ///
    /// 拼音本身干净的输入不走这条路：词图在那儿可靠，而且每次查询都生成会把按键回调拖垮。
    /// 漏网的是「英文词本身就是合法拼音、连纠错都不用」那种（`zhegeapihenhaoyong` 的 `api` 读成 `a pi`），
    /// 光看切分分不出来，要等整句评测给出「生成常态参与排序」的结论再说。
    ///
    /// 双拼 / 注音不走：它们的按键不是模型训练时见过的那套字母。
    pub(in crate::engine) fn generated_sentence_candidates(
        &self,
        best: &Segmentation,
        keys: &str,
        corrected: bool,
    ) -> Vec<Candidate> {
        if keys.len() < MIN_GENERATED_LETTERS
            || self.shuangpin.is_some()
            || self.zhuyin
            || !keys.bytes().all(|b| b.is_ascii_lowercase())
        {
            return Vec::new();
        }
        // 上面已经保证整段都是小写字母，没有 `'`，`letters()` 与字节长度可比
        let covers_all = best.letters() == keys.len();
        if best.incomplete_count() == 0 && covers_all && !corrected {
            return Vec::new();
        }
        let mut out: Vec<Candidate> = Vec::new();
        for text in self.generated_sentences(keys) {
            // 一个汉字都没有的（整段被当成英文原样抄回来）不算候选：那条路英文词表那边本来就有
            if !text.chars().any(sentence::is_han) || out.iter().any(|c| c.text == text) {
                continue;
            }
            out.push(Candidate {
                rime: None,
                text,
                kind: CandidateKind::Generated,
                syllables: Vec::new(),
                reading: None,
                translation: None,
                aux_code: None,
            });
        }
        out
    }
}
