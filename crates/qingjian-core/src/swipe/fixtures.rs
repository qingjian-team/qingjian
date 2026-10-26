//! 测试用的键位与轨迹。

use super::{SwipeKey, SwipePoint};

/// 手机全键盘的三排：键 100 × 140，第二排缩进半个键，第三排缩进一个半。
pub(super) fn keys() -> Vec<SwipeKey> {
    let rows = [("qwertyuiop", 0.0), ("asdfghjkl", 0.5), ("zxcvbnm", 1.5)];
    rows.iter()
        .enumerate()
        .flat_map(|(row, (letters, indent))| {
            letters
                .chars()
                .enumerate()
                .map(move |(column, letter)| SwipeKey {
                    letter,
                    x: (indent + column as f32 + 0.5) * 100.0,
                    y: (row as f32 + 0.5) * 140.0,
                    width: 100.0,
                    height: 140.0,
                })
        })
        .collect()
}

/// 依次直线经过这几个键的中心，每 5 像素一个点；`offset` 是整条线平移多少像素（手指没对准键中心）。
pub(super) fn trace_offset(word: &str, offset: (f32, f32)) -> Vec<SwipePoint> {
    let keys = keys();
    let centres: Vec<SwipePoint> = word
        .chars()
        .map(|letter| {
            let key = keys.iter().find(|key| key.letter == letter).unwrap();
            SwipePoint {
                x: key.x + offset.0,
                y: key.y + offset.1,
            }
        })
        .collect();
    let mut points = vec![centres[0]];
    for pair in centres.windows(2) {
        let length = (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y);
        let steps = (length / 5.0).ceil().max(1.0) as usize;
        for step in 1..=steps {
            let t = step as f32 / steps as f32;
            points.push(SwipePoint {
                x: pair[0].x + (pair[1].x - pair[0].x) * t,
                y: pair[0].y + (pair[1].y - pair[0].y) * t,
            });
        }
    }
    points
}

pub(super) fn trace(word: &str) -> Vec<SwipePoint> {
    trace_offset(word, (0.0, 0.0))
}
