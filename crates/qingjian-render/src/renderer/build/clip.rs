//! 候选文字的显示截断：只改画出来的样子，上屏的仍是全文（壳按序号上屏，不读这里的文字）。
//! 字宽按壳估矩阵列宽的同一套算：汉字一个字宽、ASCII 不到一个。

use std::borrow::Cow;

use crate::frame::{Frame, Row};

/// 一条候选最多画几个字宽，超出截尾加「…」。很长的自定义短语（整段回复、模板）不截会把候选窗撑得比屏幕还宽、
/// 一条占满；30 个字宽约是横排一页 5 条常见候选的总宽，整句候选要敲上百个字母才到这么长。
const MAX_TEXT_EMS: f32 = 30.0;

/// ASCII 字符按几个字宽算，与 Core 估列宽的 `NARROW_EMS` 一致。
const NARROW_EMS: f32 = 0.62;

/// 候选里有换行、制表符或超过 [`MAX_TEXT_EMS`] 的，换成单行截断后的帧；都不用改时原样借出。
pub(super) fn shown(frame: &Frame) -> Cow<'_, Frame> {
    let fixed: Vec<(usize, String)> = frame
        .rows
        .iter()
        .enumerate()
        .filter_map(|(i, row)| single_line(&row.text, MAX_TEXT_EMS).map(|text| (i, text)))
        .collect();
    if fixed.is_empty() {
        return Cow::Borrowed(frame);
    }
    let mut frame = frame.clone();
    for (i, text) in fixed {
        frame.rows[i] = Row {
            text,
            ..frame.rows[i].clone()
        };
    }
    Cow::Owned(frame)
}

/// 画成一行：换行画成「↵」、制表符画成「⇥」（多行短语照原样排会把一格撑高），再按 `limit` 截断；
/// 不用改时返回 `None`。
fn single_line(text: &str, limit: f32) -> Option<String> {
    if !text.contains(['\n', '\r', '\t']) {
        return clipped(text, limit);
    }
    let flat = text
        .replace("\r\n", "↵")
        .replace(['\n', '\r'], "↵")
        .replace('\t', "⇥");
    Some(clipped(&flat, limit).unwrap_or(flat))
}

/// 超过 `limit` 个字宽的截尾加「…」（「…」本身占一个字宽）；放得下返回 `None`。
pub(super) fn clipped(text: &str, limit: f32) -> Option<String> {
    let width = |c: char| if c.is_ascii() { NARROW_EMS } else { 1.0 };
    // 估宽是浮点累加，留一点余量免得刚好放得下的也被截
    let limit = limit + 0.01;
    if text.chars().map(width).sum::<f32>() <= limit {
        return None;
    }
    let mut used = 1.0;
    let kept: String = text
        .chars()
        .take_while(|&c| {
            used += width(c);
            used <= limit
        })
        .collect();
    Some(format!("{kept}…"))
}

#[cfg(test)]
mod tests {
    use super::{MAX_TEXT_EMS, clipped, shown, single_line};
    use crate::frame::{Frame, Row};

    #[test]
    fn long_candidates_are_clipped_to_the_column() {
        assert_eq!(clipped("你好", 4.0), None);
        assert_eq!(clipped("你好世界", 4.0), None);
        assert_eq!(clipped("你好世界啊", 4.0).as_deref(), Some("你好世…"));
        assert_eq!(clipped("hello", 4.0), None);
        assert_eq!(clipped("historical", 4.0).as_deref(), Some("hist…"));
    }

    #[test]
    fn multi_line_phrases_are_drawn_on_one_line() {
        assert_eq!(
            single_line("甲\r\n乙\t丙", 30.0).as_deref(),
            Some("甲↵乙⇥丙")
        );
        assert_eq!(
            single_line("一二三\n四五六", 4.0).as_deref(),
            Some("一二三…")
        );
        assert_eq!(single_line("普通候选", 30.0), None);
    }

    #[test]
    fn only_long_rows_change() {
        let long = "长".repeat(40);
        let frame = Frame {
            rows: vec![Row::plain(0, "短语"), Row::plain(1, long.clone())],
            ..Frame::default()
        };
        let shown = shown(&frame);
        assert_eq!(shown.rows[0], frame.rows[0]);
        assert_eq!(
            shown.rows[1].text,
            format!("{}…", "长".repeat(MAX_TEXT_EMS as usize - 1))
        );
        let short = Frame {
            rows: vec![Row::plain(0, "短语")],
            ..Frame::default()
        };
        assert!(matches!(
            super::shown(&short),
            std::borrow::Cow::Borrowed(_)
        ));
    }
}
