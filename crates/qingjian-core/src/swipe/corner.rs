//! 按拐点取字母：起点、终点与轨迹拐弯处各取最近的键。不查词表，直线段中间的键与 `ll` 这种重字都认不出来；
//! 是 [`ShapeDecoder`](super::ShapeDecoder) 没有词可比时（词表空、日语）的兜底。

use super::{SwipeDecoder, SwipeKey, SwipeLexicon, SwipePoint};

#[derive(Debug, Clone, Copy, Default)]
pub struct CornerDecoder;

impl SwipeDecoder for CornerDecoder {
    fn decode(&self, trace: &[SwipePoint], keys: &[SwipeKey], _: &SwipeLexicon) -> Vec<String> {
        let (Some(first), Some(last)) = (trace.first(), trace.last()) else {
            return Vec::new();
        };
        if keys.is_empty() {
            return Vec::new();
        }
        let width = keys.iter().map(|key| key.width).sum::<f32>() / keys.len() as f32;
        let mut stops = vec![*first];
        stops.extend(corners(trace, width * REACH));
        stops.push(*last);

        let mut letters = String::new();
        for stop in stops {
            let nearest = keys
                .iter()
                .min_by(|a, b| a.distance(stop).total_cmp(&b.distance(stop)))
                .map(|key| key.letter.to_ascii_lowercase());
            if let Some(letter) = nearest
                && !letters.ends_with(letter)
            {
                letters.push(letter);
            }
        }
        vec![letters]
    }
}

/// 拐弯处：每一点往回、往前各量出 `reach` 长的一段，两段方向差过 [`CORNER_DEGREES`] 算在拐；连着的一串拐点取拐得最急的那个。
fn corners(trace: &[SwipePoint], reach: f32) -> Vec<SwipePoint> {
    let mut along = Vec::with_capacity(trace.len());
    let mut total = 0.0;
    for (i, point) in trace.iter().enumerate() {
        if i > 0 {
            total += length(trace[i - 1], *point);
        }
        along.push(total);
    }

    let mut found = Vec::new();
    // 正在跟的一串拐点里最急的那个：（下标，角度）
    let mut best: Option<(usize, f32)> = None;
    for i in 0..trace.len() {
        let before = (0..i).rev().find(|&j| along[i] - along[j] >= reach);
        let after = (i + 1..trace.len()).find(|&k| along[k] - along[i] >= reach);
        let angle = match (before, after) {
            (Some(j), Some(k)) => turn(trace[j], trace[i], trace[k]),
            _ => 0.0,
        };
        if angle >= CORNER_DEGREES {
            if best.is_none_or(|(_, sharpest)| angle > sharpest) {
                best = Some((i, angle));
            }
        } else if let Some((index, _)) = best.take() {
            found.push(trace[index]);
        }
    }
    if let Some((index, _)) = best {
        found.push(trace[index]);
    }
    found
}

fn length(a: SwipePoint, b: SwipePoint) -> f32 {
    (b.x - a.x).hypot(b.y - a.y)
}

/// 从 a 经 b 到 c 拐了多少度（0 是直走，180 是折返）。
fn turn(a: SwipePoint, b: SwipePoint, c: SwipePoint) -> f32 {
    let (ux, uy) = (b.x - a.x, b.y - a.y);
    let (vx, vy) = (c.x - b.x, c.y - b.y);
    let norms = ux.hypot(uy) * vx.hypot(vy);
    if norms <= f32::EPSILON {
        return 0.0;
    }
    ((ux * vx + uy * vy) / norms)
        .clamp(-1.0, 1.0)
        .acos()
        .to_degrees()
}

/// 量拐弯用的那段有多长，按平均键宽算：太短手抖也算拐，太长急弯被抹平。
const REACH: f32 = 0.4;

/// 方向差过这么多度算拐弯。
const CORNER_DEGREES: f32 = 50.0;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swipe::fixtures::{keys, trace};

    fn decode(word: &str) -> String {
        CornerDecoder
            .decode(&trace(word), &keys(), &SwipeLexicon::default())
            .remove(0)
    }

    #[test]
    fn straight_line_keeps_only_the_ends() {
        assert_eq!(decode("qp"), "qp");
        assert_eq!(decode("hi"), "hi");
    }

    #[test]
    fn corners_add_letters() {
        assert_eq!(decode("the"), "the");
        assert_eq!(decode("cat"), "cat");
        assert_eq!(decode("zhe"), "zhe");
    }

    #[test]
    fn a_single_point_is_its_key() {
        let point = SwipePoint { x: 52.0, y: 210.0 };
        assert_eq!(
            CornerDecoder.decode(&[point], &keys(), &SwipeLexicon::default()),
            vec!["a".to_owned()]
        );
    }

    #[test]
    fn nothing_to_decode() {
        let lexicon = SwipeLexicon::default();
        assert!(CornerDecoder.decode(&[], &keys(), &lexicon).is_empty());
        assert!(CornerDecoder.decode(&trace("hi"), &[], &lexicon).is_empty());
    }
}
