//! 协议帧（[`qingjian_platform::protocol::Frame`]）→ 渲染器要的展示数据（[`crate::Frame`]）。
//!
//! 各平台壳都拿同一份协议帧：Windows 的 Server 进程自绘、Windows TSF DLL 在系统界面宿主里进程内自绘、
//! 以及别的端各自贴图，展示形态都从这里换算，免得同一套「标题行 / 序号 / 译词 / 页码」在几处各写一遍。
//!
//! 这里只有展示形态，不含配色、字号：那些归 [`crate::Theme`]。

use qingjian_platform::protocol::{Frame, PreeditKind};
use qingjian_platform::{Appearance, LayoutMode};

use crate::frame::{Frame as RenderFrame, Mode, Preedit, PreeditSegment, PreeditStyle, Row, Tone};

/// 一帧要显示的全部内容，由协议帧换算而来；`render_frame` 再换成渲染器的帧。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CandidateView {
    /// 顶部拼音行的各段。
    pub preedit: Vec<(String, PreeditKind)>,

    /// 光标在拼音行里的字符位置。
    pub cursor: usize,

    /// 候选行。
    pub rows: Vec<Row>,

    /// 高亮行下标（页内）；`usize::MAX` 表示不高亮。
    pub highlight: usize,

    /// 页码，只有多页时有。
    pub footer: Option<String>,

    /// 往前 / 往后还有页：主题把翻不动的箭头画淡。
    pub pages_around: (bool, bool),

    /// 整句补全，画在拼音行右侧。
    pub sentence: Option<String>,

    /// 屏幕提示（删候选后的「已删除…」），画在拼音行下方。
    pub notice: Option<String>,

    /// 候选排布。
    pub layout: LayoutMode,

    /// 矩阵每行几格（`> 0` 是横排展开成矩阵）；单行为 0。
    pub columns: usize,

    /// 矩阵各列留几个候选字宽（整份候选估的，移动高亮时不变，窗口才不跳）。
    pub column_ems: Vec<f32>,

    /// 外观；`System` 由窗口按系统深浅色解析。
    pub appearance: Appearance,

    /// 候选上是否显示辅码（随帧下发的 `[general] aux_code_show`）。
    pub show_code: bool,

    /// 译词旁是否显示词性标注（随帧下发的 `[general] translation_pos`）。
    pub show_pos: bool,

    /// 英文候选的 `reading` 是音标、用淡色画（随帧下发的 `[general] show_english_pronunciation`）。
    pub english_reading_ipa: bool,

    /// 输入状态，给主题显示中 / 英、简 / 繁等。
    pub mode: Mode,
}

impl CandidateView {
    /// 按协议帧换上这一帧的内容。
    pub fn set(&mut self, frame: &Frame, mode: Mode) {
        self.mode = mode;
        self.layout = frame.layout;
        self.appearance = frame.appearance;
        self.show_code = frame.aux_code_show;
        self.show_pos = frame.translation_pos;
        self.english_reading_ipa = frame.english_reading_ipa;
        self.preedit = window_preedit(frame);
        self.cursor = frame
            .window_preedit
            .as_ref()
            .map_or(frame.cursor, |window| window.1);
        self.columns = frame.columns;
        self.column_ems = frame.column_ems.clone();
        let grid = frame.columns > 0;
        // 矩阵里序号只标在高亮所在那一行（数字键选的就是它），空位什么都不画；单行照旧全标
        let highlight_row = grid.then(|| frame.highlight / frame.columns);
        self.rows = frame
            .candidates
            .items
            .iter()
            .enumerate()
            .map(|(i, candidate)| {
                let mut row = row_from_candidate(
                    if grid { i % frame.columns } else { i },
                    candidate,
                    self.show_code,
                    self.show_pos,
                    self.english_reading_ipa,
                );
                row.diff = frame.differs.get(i).cloned().unwrap_or_default();
                if grid {
                    let labelled =
                        Some(i / frame.columns) == highlight_row && !candidate.text.is_empty();
                    if !labelled {
                        row.index = String::new();
                    }
                }
                row
            })
            .collect();
        self.highlight = frame.highlight;
        self.footer =
            (frame.page_count > 1).then(|| format!("{}/{}", frame.page + 1, frame.page_count));
        self.pages_around = (frame.page > 0, frame.page + 1 < frame.page_count);
        self.sentence = frame.sentence.clone();
        self.notice = frame.notice.clone();
    }

    /// 渲染器要的帧。提示（删了什么词）在渲染器里画在拼音行右侧，与 macOS 一致。
    pub fn render_frame(&self) -> RenderFrame {
        let preedit = (!self.preedit.is_empty()).then(|| Preedit {
            segments: self
                .preedit
                .iter()
                .map(|(text, kind)| PreeditSegment {
                    text: text.clone(),
                    style: match kind {
                        PreeditKind::Typed => PreeditStyle::Typed,
                        PreeditKind::Rest => PreeditStyle::Rest,
                        PreeditKind::Corrected => PreeditStyle::Struck,
                        PreeditKind::AuxCode => PreeditStyle::AuxCode,
                    },
                })
                .collect(),
            cursor: self.cursor,
        });
        RenderFrame {
            preedit,
            rows: self.rows.clone(),
            // 协议里 usize::MAX 表示不高亮。
            highlighted: (self.highlight != usize::MAX).then_some(self.highlight),
            columns: self.columns,
            column_ems: self.column_ems.clone(),
            footer: self.footer.clone(),
            previous_page: self.pages_around.0,
            next_page: self.pages_around.1,
            sentence: self.sentence.clone(),
            status: self.notice.clone(),
            mode: self.mode.clone(),
            reversed: false,
        }
    }
}

/// 窗口顶部要画的拼音行：`[general] preedit` 配成「只在行内」时为空（拼音已经在应用里）；
/// 整句补全仍画在这一行上，所以空行时窗口仍可能留出这条线。
fn window_preedit(frame: &Frame) -> Vec<(String, PreeditKind)> {
    if !frame.preedit_mode.in_window() {
        return Vec::new();
    }
    // 带调拼音时窗口里画另一串，应用里的行内拼音仍是 `preedit`
    frame
        .window_preedit
        .as_ref()
        .map_or(&frame.preedit, |window| &window.0)
        .iter()
        .map(|segment| (segment.text.clone(), segment.kind))
        .collect()
}

/// 候选窗口的一行：[`qingjian_core::Candidate`] → 渲染器的 [`Row`]（序号、候选词、annotation 片段），
/// 与 macOS 端 `candidates/row.rs` 一致；Windows 的 GDI 画法也用同一个类型。
///
/// `position` 是页内下标（从 0 起）。`show_code` 是 `[general] aux_code_show`：
/// 打开且候选带码时，码用方括号括起来紧跟在候选词后面（`鹤[rbm]`），不进 annotation。
/// `show_pos` 是 `[general] translation_pos`：关掉时译词保留、词性段（v. / n. / adj.）不画。
/// `english_reading_ipa` 是 `[general] show_english_pronunciation`：英文候选的 `reading` 是音标
/// （`/ˈkɑrɡoʊ/`）而不是问字模式的带调拼音，用淡色画，免得跟后面同色的译文混在一起。
pub fn row_from_candidate(
    position: usize,
    candidate: &qingjian_core::Candidate,
    show_code: bool,
    show_pos: bool,
    english_reading_ipa: bool,
) -> Row {
    use qingjian_core::CandidateKind;

    let code = candidate
        .aux_code
        .as_ref()
        .filter(|_| show_code)
        .map(|code| format!("[{code}]"));
    let mut annotation = Vec::new();
    // 英文候选的 reading 是音标（辅助信息，排到译文之后、淡色）；其余候选的 reading 是问字模式
    // 答案的带调拼音，那是候选本身的内容，留在最前面、用译词色。
    let english_ipa = candidate
        .reading
        .as_ref()
        .filter(|_| english_reading_ipa && candidate.kind == CandidateKind::English)
        .cloned();
    if let Some(reading) = candidate.reading.as_ref().filter(|_| english_ipa.is_none()) {
        annotation.push((reading.clone(), Tone::Gloss));
    }
    if let Some(translation) = &candidate.translation {
        for (i, sense) in translation.senses().iter().enumerate() {
            if i > 0 || !annotation.is_empty() {
                annotation.push((" · ".to_owned(), Tone::Separator));
            }
            if show_pos && let Some(pos) = sense.part_of_speech {
                annotation.push((format!("{pos} "), Tone::Pos));
            }
            let tone = if sense.fresh {
                Tone::Fresh
            } else {
                Tone::Gloss
            };
            for segment in sense.furigana() {
                annotation.push((segment.text, tone));
                if let Some(reading) = segment.reading {
                    annotation.push((format!("({reading})"), Tone::Faint));
                }
            }
            // 译文的音标跟在译词后面、不带括号（`hello /həˈloʊ/`）；括号是日语假名的写法
            if let Some(pronunciation) = &sense.pronunciation {
                annotation.push((format!(" {pronunciation}"), Tone::Faint));
            }
        }
    }
    // 英文候选自己的音标也排在译文之后，前面补一个空格与译文分开
    if let Some(reading) = english_ipa {
        if !annotation.is_empty() {
            annotation.push((" ".to_owned(), Tone::Separator));
        }
        annotation.push((reading, Tone::Faint));
    }
    Row {
        index: (position + 1).to_string(),
        text: candidate.text.clone(),
        diff: Vec::new(),
        code,
        annotation,
        cloud: candidate.kind == qingjian_core::CandidateKind::Cloud,
    }
}

#[cfg(test)]
mod tests {
    use qingjian_core::{Language, PartOfSpeech, Sense, Translation};
    use qingjian_platform::PreeditMode;
    use qingjian_platform::protocol::PreeditSegment;

    use super::*;

    fn candidate_with_gloss() -> qingjian_core::Candidate {
        qingjian_core::Candidate {
            text: "开发".to_owned(),
            kind: qingjian_core::CandidateKind::Chinese,
            syllables: vec!["kai".to_owned(), "fa".to_owned()],
            reading: None,
            translation: Some(Translation::new(
                Language::English,
                vec![Sense {
                    part_of_speech: Some(PartOfSpeech::Verb),
                    text: "develop".to_owned(),
                    reading: None,
                    pronunciation: None,
                    fresh: false,
                }],
            )),
            aux_code: None,
        }
    }

    #[test]
    fn part_of_speech_follows_the_frame_switch() {
        let on = row_from_candidate(0, &candidate_with_gloss(), false, true, false);
        let joined: String = on
            .annotation
            .iter()
            .map(|(text, _)| text.as_str())
            .collect();
        assert!(joined.contains("v. "), "{joined}");
        assert!(joined.contains("develop"), "{joined}");

        // 关掉后词性段消失，译词与分隔结构原样保留
        let off = row_from_candidate(0, &candidate_with_gloss(), false, false, false);
        let joined: String = off
            .annotation
            .iter()
            .map(|(text, _)| text.as_str())
            .collect();
        assert!(!joined.contains("v. "), "{joined}");
        assert!(joined.contains("develop"), "{joined}");
        assert_eq!(on.annotation.len() - 1, off.annotation.len());
    }

    #[test]
    fn english_reading_goes_after_the_translation() {
        let mut candidate = candidate_with_gloss();
        candidate.text = "cargo".to_owned();
        candidate.kind = qingjian_core::CandidateKind::English;
        candidate.reading = Some("/ˈkɑrˌɡoʊ/".to_owned());
        let row = row_from_candidate(0, &candidate, false, true, true);
        let joined: String = row
            .annotation
            .iter()
            .map(|(text, _)| text.as_str())
            .collect();
        assert_eq!(joined, "v. develop /ˈkɑrˌɡoʊ/");
        let tone = row.annotation.last().map(|(_, tone)| *tone);
        assert_eq!(tone, Some(Tone::Faint));
    }

    /// 只有「只在行内」不给窗口拼音行；另两档窗口都得画。
    #[test]
    fn window_keeps_the_pinyin_row_unless_inline_only() {
        for (mode, expected) in [
            (PreeditMode::Both, 1),
            (PreeditMode::Window, 1),
            (PreeditMode::Inline, 0),
        ] {
            let frame = Frame {
                preedit: vec![PreeditSegment {
                    text: "ni'hao".to_owned(),
                    kind: PreeditKind::Typed,
                }],
                preedit_mode: mode,
                ..Frame::default()
            };
            assert_eq!(window_preedit(&frame).len(), expected, "{mode:?}");
        }
    }
}
