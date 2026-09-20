//! 壳的帧类型 → 渲染器的帧类型。两边字段一一对应，spike 定型后壳直接用渲染器的类型，这层就没了。

use crate::candidates::frame::Frame;
use crate::candidates::preedit::{Preedit, PreeditStyle};
use crate::candidates::row::{Row, Tone};

pub(super) fn frame(frame: &Frame) -> qingjian_render::Frame {
    qingjian_render::Frame {
        preedit: frame.preedit.as_ref().map(preedit),
        rows: frame.rows.iter().map(row).collect(),
        highlighted: Some(frame.highlighted),
        footer: frame.footer.clone(),
        sentence: frame.sentence.clone(),
        status: frame.status.clone(),
    }
}

fn preedit(preedit: &Preedit) -> qingjian_render::Preedit {
    qingjian_render::Preedit {
        segments: preedit
            .segments
            .iter()
            .map(|segment| qingjian_render::PreeditSegment {
                text: segment.text.clone(),
                style: match segment.style {
                    PreeditStyle::Typed => qingjian_render::PreeditStyle::Typed,
                    PreeditStyle::Rest => qingjian_render::PreeditStyle::Rest,
                    PreeditStyle::Struck => qingjian_render::PreeditStyle::Struck,
                },
            })
            .collect(),
        cursor: preedit.cursor,
    }
}

fn row(row: &Row) -> qingjian_render::Row {
    qingjian_render::Row {
        index: row.index.clone(),
        text: row.text.clone(),
        code: None,
        annotation: row
            .annotation
            .iter()
            .map(|(text, tone)| {
                let tone = match tone {
                    Tone::Gloss => qingjian_render::Tone::Gloss,
                    Tone::Fresh => qingjian_render::Tone::Fresh,
                    Tone::Faint => qingjian_render::Tone::Faint,
                };
                (text.clone(), tone)
            })
            .collect(),
        cloud: row.cloud,
    }
}
