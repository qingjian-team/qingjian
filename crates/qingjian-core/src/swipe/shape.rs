//! 按词表打分的解码：每个候选词依次连起各字母键的中心是它的「理想路径」，与实际轨迹比形状与位置，再加上词频先验。
//!
//! 只比起点、终点附近那几个键开头结尾的词（词表按首尾字母分了桶），按「先验减起终点差」这个总分上限从高到低逐个打分，
//! 上限已经比不过前几名就停，一个词算到一半超出也停；随包词表上一次解码在电脑上不到 1 ms，最慢几毫秒。
//! 形状差是下面几项距离平方之和，距离都按键的宽高归一（一个键宽 = 1）：
//! - 两条线各按长度重采样成同样多的点，逐点比的均方：顺序、位置、比例都要对得上；
//! - 词的每个字母按顺序在轨迹上找最近的点（动态规划），均方：直线划过的字母也算经过；
//! - 轨迹离理想路径最远的那一点：轨迹拐去的地方词里也得有字母（`nihao` 不是 `niao`）；
//! - 起点、终点各自离首尾字母多远：终点差一个键（`hello` 与 `help`）比中间差一点要紧得多。
//!
//! 同一个键连着两次（`ll`）在路径上是同一个点，靠词频分：`hello` 比 `helo` 常用。设计见 `docs/design/voice-and-swipe.md`「滑行」。

use super::{CornerDecoder, SwipeDecoder, SwipeKey, SwipeLexicon, SwipePoint};

#[derive(Debug, Clone, Copy, Default)]
pub struct ShapeDecoder;

/// 键宽高归一后的坐标。
type Point = (f32, f32);

impl SwipeDecoder for ShapeDecoder {
    fn decode(
        &self,
        trace: &[SwipePoint],
        keys: &[SwipeKey],
        lexicon: &SwipeLexicon,
    ) -> Vec<String> {
        if trace.is_empty() || keys.is_empty() {
            return Vec::new();
        }
        if lexicon.is_empty() {
            return CornerDecoder.decode(trace, keys, lexicon);
        }
        let width = keys.iter().map(|key| key.width).sum::<f32>() / keys.len() as f32;
        let height = keys.iter().map(|key| key.height).sum::<f32>() / keys.len() as f32;
        let unit = |x: f32, y: f32| (x / width.max(1.0), y / height.max(1.0));
        let mut centres: [Option<Point>; 26] = [None; 26];
        for key in keys {
            let letter = key.letter.to_ascii_lowercase();
            if letter.is_ascii_lowercase() {
                centres[(letter as u8 - b'a') as usize] = Some(unit(key.x, key.y));
            }
        }
        let points: Vec<Point> = trace.iter().map(|point| unit(point.x, point.y)).collect();
        let trace_length = path_length(&points);
        let mut sampled = Vec::with_capacity(SAMPLES);
        resample(&points, &mut sampled);
        let near = |at: Point| -> Vec<u8> {
            (b'a'..=b'z')
                .filter(|&letter| {
                    centres[(letter - b'a') as usize]
                        .is_some_and(|centre| distance(centre, at) <= END_REACH)
                })
                .collect()
        };
        let starts = near(points[0]);
        let ends = near(points[points.len() - 1]);

        // 起点、终点的差只看首尾字母，先验减去它是这个词总分的上限：按上限从高到低比，上限已经不如第几名就停
        let mut bounded: Vec<(f32, &str)> = Vec::new();
        for &first in &starts {
            for &last in &ends {
                let ends_cost = squared(sampled[0], centre_of(&centres, first))
                    + squared(sampled[SAMPLES - 1], centre_of(&centres, last));
                bounded.extend(lexicon.ending(first, last).map(|(code, prior)| {
                    (prior * PRIOR_WEIGHT - ends_cost * MISMATCH_WEIGHT, code)
                }));
            }
        }
        bounded.sort_by(|a, b| b.0.total_cmp(&a.0));

        let mut best: Vec<(f32, &str)> = Vec::with_capacity(RESULTS + 1);
        let (mut path, mut ideal, mut costs) = (Vec::new(), Vec::new(), Vec::new());
        for (bound, code) in bounded {
            // 进得了前几名、又不比第一名差太远的最低分
            let floor = match (best.first(), best.get(RESULTS - 1)) {
                (Some(&(top, _)), Some(&(last, _))) => last.max(top - SPREAD),
                (Some(&(top, _)), None) => top - SPREAD,
                _ => f32::NEG_INFINITY,
            };
            if bound < floor {
                break;
            }
            if !letter_path(code, &centres, &mut path) {
                continue;
            }
            let length = path_length(&path);
            if length > trace_length * LENGTH_SLACK + 1.0
                || length * LENGTH_SLACK + 1.0 < trace_length
            {
                continue;
            }
            let budget = (bound - floor) / MISMATCH_WEIGHT;
            let Some(rest) = mismatch(&sampled, &path, budget, &mut ideal, &mut costs) else {
                continue;
            };
            let score = bound - rest * MISMATCH_WEIGHT;
            let at = best.partition_point(|&(other, other_code)| {
                other > score || (other == score && other_code < code)
            });
            if at < RESULTS {
                best.insert(at, (score, code));
                best.truncate(RESULTS);
            }
        }
        if best.is_empty() {
            return CornerDecoder.decode(trace, keys, lexicon);
        }
        best.into_iter().map(|(_, code)| code.to_owned()).collect()
    }
}

fn centre_of(centres: &[Option<Point>; 26], letter: u8) -> Point {
    centres[(letter - b'a') as usize].unwrap_or_default()
}

/// 重采样过的轨迹与词的理想路径 `path` 除起点、终点之外的形状差（见文件头）；算的过程中超过 `budget` 就回 `None`，
/// 便宜的项先算。`ideal`、`costs` 是整次解码复用的缓冲。
fn mismatch(
    sampled: &[Point],
    path: &[Point],
    budget: f32,
    ideal: &mut Vec<Point>,
    costs: &mut Vec<f32>,
) -> Option<f32> {
    resample(path, ideal);
    let mut total = sampled
        .iter()
        .zip(ideal.iter())
        .map(|(&a, &b)| squared(a, b))
        .sum::<f32>()
        / SAMPLES as f32;
    if total > budget {
        return None;
    }
    total += coverage(path, sampled, costs);
    if total > budget {
        return None;
    }
    let wander = sampled
        .iter()
        .map(|&point| polyline_distance(point, path))
        .fold(0.0, f32::max);
    total += wander * wander;
    (total <= budget).then_some(total)
}

/// 词的理想路径（各字母键中心，连着同一个键的只留一个点）写进 `path`；有字母不在键盘上回假。
fn letter_path(code: &str, centres: &[Option<Point>; 26], path: &mut Vec<Point>) -> bool {
    path.clear();
    for byte in code.bytes() {
        let Some(centre) = centres[(byte - b'a') as usize] else {
            return false;
        };
        if path.last() != Some(&centre) {
            path.push(centre);
        }
    }
    true
}

/// 按顺序给每个点在轨迹上找一个不早于前一个的落点，落点距离平方之和最小时的均值。
fn coverage(path: &[Point], sampled: &[Point], costs: &mut Vec<f32>) -> f32 {
    costs.resize(sampled.len(), 0.0);
    for (cost, &point) in costs.iter_mut().zip(sampled) {
        *cost = squared(path[0], point);
    }
    for &letter in &path[1..] {
        let mut best_before = f32::INFINITY;
        for (cost, &point) in costs.iter_mut().zip(sampled) {
            best_before = best_before.min(*cost);
            *cost = best_before + squared(letter, point);
        }
    }
    costs.iter().copied().fold(f32::INFINITY, f32::min) / path.len() as f32
}

/// 沿折线按长度等分取 [`SAMPLES`] 个点写进 `out`；长度为零（一个点或原地不动）就全是起点。
fn resample(points: &[Point], out: &mut Vec<Point>) {
    out.clear();
    let total = path_length(points);
    if total <= f32::EPSILON {
        out.resize(SAMPLES, points[0]);
        return;
    }
    let step = total / (SAMPLES - 1) as f32;
    out.push(points[0]);
    let mut segment = 0;
    let mut walked = 0.0;
    for index in 1..SAMPLES - 1 {
        let target = step * index as f32;
        while segment + 2 < points.len()
            && walked + distance(points[segment], points[segment + 1]) < target
        {
            walked += distance(points[segment], points[segment + 1]);
            segment += 1;
        }
        let (a, b) = (points[segment], points[segment + 1]);
        let span = distance(a, b);
        let t = if span <= f32::EPSILON {
            0.0
        } else {
            ((target - walked) / span).clamp(0.0, 1.0)
        };
        out.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
    }
    out.push(points[points.len() - 1]);
}

fn path_length(points: &[Point]) -> f32 {
    points
        .windows(2)
        .map(|pair| distance(pair[0], pair[1]))
        .sum()
}

/// 点到折线（一个点时就是到那个点）的最近距离。
fn polyline_distance(point: Point, path: &[Point]) -> f32 {
    if path.len() == 1 {
        return distance(point, path[0]);
    }
    path.windows(2)
        .map(|pair| segment_distance(point, pair[0], pair[1]))
        .fold(f32::INFINITY, f32::min)
}

fn segment_distance(point: Point, a: Point, b: Point) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let squared = dx * dx + dy * dy;
    let t = if squared <= f32::EPSILON {
        0.0
    } else {
        (((point.0 - a.0) * dx + (point.1 - a.1) * dy) / squared).clamp(0.0, 1.0)
    };
    distance(point, (a.0 + dx * t, a.1 + dy * t))
}

fn squared(a: Point, b: Point) -> f32 {
    (b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)
}

fn distance(a: Point, b: Point) -> f32 {
    (b.0 - a.0).hypot(b.1 - a.1)
}

/// 两条线各取多少个点比。
const SAMPLES: usize = 48;

/// 词的首尾字母键离轨迹起点、终点多远以内（键宽）才比：起落在键边上时连邻键也算上。
const END_REACH: f32 = 1.1;

/// 理想路径与轨迹的长度差多少还比：长的不超过短的这么多倍再多一个键宽。抄近路、划过头都常见，放得宽。
const LENGTH_SLACK: f32 = 1.8;

/// 形状差（键宽²）的权重；与词频先验（log10）相减成总分。
const MISMATCH_WEIGHT: f32 = 5.0;

/// 词频先验的权重：词频差十倍抵多少形状差。
const PRIOR_WEIGHT: f32 = 1.0;

/// 比第一名低这么多分的不要了（也不当备选）：最长的词也不用每个都比完。
const SPREAD: f32 = 6.0;

/// 回几个结果。
const RESULTS: usize = 5;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swipe::fixtures::{keys, trace, trace_offset};

    /// 英文：常用词加上形状相近的干扰（`te` 是 `the` 的直线版，`hell`、`helo` 与 `hello` 只差尾巴或重字）。
    fn english() -> SwipeLexicon {
        SwipeLexicon::new(
            [
                ("the", 7.73),
                ("te", 3.5),
                ("tie", 4.6),
                ("toe", 4.2),
                ("he", 6.8),
                ("hello", 4.72),
                ("hell", 5.1),
                ("helo", 1.5),
                ("help", 5.6),
                ("ho", 4.0),
                ("world", 6.0),
                ("word", 5.6),
                ("good", 6.2),
                ("god", 5.5),
                ("you", 7.3),
                ("yo", 4.5),
                ("a", 7.4),
                ("i", 7.0),
            ]
            .map(|(code, prior)| (code.to_owned(), prior)),
        )
    }

    /// 拼音：音节串与词，`niao`、`nia` 是 `nihao` 划过的字母的子集，`zhongguo` 也比 `zhonggu`、`zhong` 长。
    fn pinyin() -> SwipeLexicon {
        SwipeLexicon::new(
            [
                ("ni", 6.5),
                ("hao", 6.0),
                ("niao", 4.8),
                ("nihao", 4.5),
                ("nia", 2.0),
                ("zhong", 6.0),
                ("zhongguo", 5.0),
                ("zhonggu", 1.0),
                ("zhuo", 4.5),
                ("zhe", 6.2),
                ("ze", 5.0),
                ("women", 5.4),
                ("wo", 6.6),
                ("wen", 5.5),
                ("xiexie", 4.6),
                ("xie", 5.8),
                ("shi", 6.9),
                ("si", 5.6),
            ]
            .map(|(code, prior)| (code.to_owned(), prior)),
        )
    }

    fn first(word: &str, lexicon: &SwipeLexicon) -> String {
        ShapeDecoder
            .decode(&trace(word), &keys(), lexicon)
            .remove(0)
    }

    #[test]
    fn english_words() {
        let lexicon = english();
        for word in [
            "the", "hello", "hell", "help", "world", "word", "good", "you",
        ] {
            assert_eq!(first(word, &lexicon), word);
        }
    }

    #[test]
    fn pinyin_syllables_and_words() {
        let lexicon = pinyin();
        for word in [
            "nihao", "zhongguo", "zhong", "women", "xiexie", "zhe", "shi",
        ] {
            assert_eq!(first(word, &lexicon), word);
        }
    }

    #[test]
    fn letters_passed_in_a_straight_line_count() {
        // q 到 p 一条直线划过 wertyuio：quip 的 u、i 都在线上，比两头的 qp 常用；qwertyuiop 一样形状但没那么常用
        let lexicon = SwipeLexicon::new(
            [("qp", 1.0), ("quip", 3.0), ("qwertyuiop", 2.0)].map(|(c, p)| (c.to_owned(), p)),
        );
        assert_eq!(first("quip", &lexicon), "quip");
        assert_eq!(first("qwertyuiop", &lexicon), "quip");
    }

    #[test]
    fn off_centre_traces_still_decode() {
        let lexicon = english();
        let keys = keys();
        for offset in [(30.0, 0.0), (-25.0, 35.0), (0.0, -40.0)] {
            for word in ["the", "hello", "world"] {
                let decoded = ShapeDecoder.decode(&trace_offset(word, offset), &keys, &lexicon);
                assert_eq!(decoded[0], word, "{offset:?}");
            }
        }
    }

    #[test]
    fn gives_several_results_best_first() {
        let decoded = ShapeDecoder.decode(&trace("hello"), &keys(), &english());
        assert_eq!(decoded[0], "hello");
        assert!(decoded.contains(&"helo".to_owned()));
        assert!(decoded.len() <= RESULTS);
    }

    #[test]
    fn a_single_point_is_a_one_letter_word() {
        let point = SwipePoint { x: 52.0, y: 210.0 };
        assert_eq!(ShapeDecoder.decode(&[point], &keys(), &english())[0], "a");
    }

    #[test]
    fn without_words_falls_back_to_corners() {
        let empty = SwipeLexicon::default();
        assert_eq!(
            ShapeDecoder.decode(&trace("the"), &keys(), &empty),
            vec!["the".to_owned()]
        );
        // 起终点附近没有词：也退回拐点
        assert_eq!(
            ShapeDecoder.decode(&trace("qp"), &keys(), &english()),
            vec!["qp".to_owned()]
        );
    }

    #[test]
    fn nothing_to_decode() {
        assert!(ShapeDecoder.decode(&[], &keys(), &english()).is_empty());
        assert!(
            ShapeDecoder
                .decode(&trace("hi"), &[], &english())
                .is_empty()
        );
    }
}
