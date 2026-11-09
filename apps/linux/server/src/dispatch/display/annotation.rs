//! Linux 面板的显示注释；原始译文保持不变，译词快捷键只提交原文。
use qingjian_core::Sense;
use qingjian_platform::protocol::Frame;
use serde_json::{Value, json};

pub(super) fn annotate_frame(frame: &Frame, value: &mut Value) {
    let Some(items) = value["candidates"]["items"].as_array_mut() else {
        return;
    };
    for (candidate, item) in frame.candidates.items.iter().zip(items) {
        let Some(translation) = &candidate.translation else {
            continue;
        };
        for (index, sense) in translation.senses().iter().enumerate() {
            item["translation"]["senses"][index]["annotation"] =
                json!(annotation(sense, frame.translation_pos));
        }
    }
}

fn annotation(sense: &Sense, part_of_speech: bool) -> String {
    if sense.text.is_empty() {
        return String::new();
    }
    let mut text = String::new();
    if let Some(pos) = sense.part_of_speech.filter(|_| part_of_speech) {
        text.push_str(pos.abbreviation());
        text.push(' ');
    }
    for segment in sense.furigana() {
        text.push_str(&segment.text);
        if let Some(reading) = segment.reading {
            text.push('(');
            text.push_str(&reading);
            text.push(')');
        }
    }
    text
}
