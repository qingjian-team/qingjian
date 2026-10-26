//! 候选条：把帧里的候选换成渲染器的行，画成位图交给壳贴（见 `docs/design/rendering.md`）。

mod hit;
mod meta;
mod painter;

use qingjian_render::{Row, Tone};

use crate::frame::FrameCandidate;

pub use self::hit::StripHit;
pub use self::meta::StripMeta;
pub use self::painter::StripPainter;

/// 译文一行最宽（点），超出的截断加省略号；与主题里候选条的译文字号一起定。
const MAX_GLOSS_WIDTH: f32 = 120.0;

/// 候选条译文的字号（点），与主题 `strip_cell` 里的一致，截断时量宽度用。
const GLOSS_SIZE: f32 = 11.5;

/// 每个候选只带第一条译文（词性 + 释义），长的截断；更多的译文在长按气泡里。`text_scale` 是设置里的字号档，宽度与字号一起放大。
fn rows(
    candidates: &[FrameCandidate],
    text_scale: f32,
    mut measure: impl FnMut(&str, f32) -> f32,
) -> Vec<Row> {
    let (room, size) = (MAX_GLOSS_WIDTH * text_scale, GLOSS_SIZE * text_scale);
    candidates
        .iter()
        .enumerate()
        .map(|(i, candidate)| {
            let mut row = Row::plain(i, candidate.text.clone());
            row.index.clear();
            row.cloud = candidate.cloud;
            if let Some(sense) = candidate.senses.first() {
                let pos = sense.pos.map(|pos| format!("{pos} ")).unwrap_or_default();
                let room = room - measure(&pos, size);
                let gloss = fit(&sense.text, room, |text| measure(text, size));
                if !pos.is_empty() {
                    row.annotation.push((pos, Tone::Pos));
                }
                let tone = if sense.fresh {
                    Tone::Fresh
                } else {
                    Tone::Gloss
                };
                row.annotation.push((gloss, tone));
            }
            row
        })
        .collect()
}

/// 放不下 `room` 点宽就从尾部去字加「…」。
fn fit(text: &str, room: f32, mut width: impl FnMut(&str) -> f32) -> String {
    if width(text) <= room {
        return text.to_owned();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let candidate: String = chars.iter().collect::<String>() + "…";
        if width(&candidate) <= room {
            return candidate;
        }
    }
    String::from("…")
}

/// 文本里有没有要按彩色 emoji 画的字：象形符号区、杂项技术符号（⌚⌛⏰ 这些默认就是 emoji，不带 FE0F）、
/// 杂项符号、变体选择符 U+FE0F。
fn has_emoji(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(c as u32,
            0x1F000..=0x1FAFF | 0x2300..=0x23FF | 0x2600..=0x27BF | 0x2B00..=0x2BFF | 0xFE0F)
    })
}

#[cfg(test)]
mod tests;
