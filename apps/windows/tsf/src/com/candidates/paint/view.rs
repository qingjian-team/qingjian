//! 一帧要画的全部内容：顶部拼音行、候选行、页码、整句补全与提示，由协议帧换算。
//!
//! 换算规则与 Server 候选窗的 [`RenderData`](../../../../server/src/ui/candidates/render_data.rs) 一致：
//! 帧里的「序号怎么标、矩阵空位怎么办」这些展示细节都在这里定，画法只管量尺寸与落笔。

use qingjian_platform::LayoutMode;
use qingjian_platform::protocol::{Frame, PreeditKind};

use super::row::{self, Row};

/// 内容区里一块收鼠标的矩形（像素，左上角为内容区原点）。
#[derive(Debug, Clone)]
pub(crate) struct HitRegion {
    pub(crate) x: f32,

    pub(crate) y: f32,

    pub(crate) width: f32,

    pub(crate) height: f32,

    pub(crate) target: HitTarget,
}

impl HitRegion {
    pub(crate) fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}

/// 点中的是什么。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HitTarget {
    /// 这一页第 `n` 个候选（从 0 起）。
    Candidate(usize),

    /// 第 `candidate` 个候选右侧第 `sense` 条译词（从 0 起）。
    Translation { candidate: usize, sense: usize },
}

/// 一次绘制要用的展示数据。
#[derive(Debug, Clone, Default)]
pub(crate) struct View {
    /// 顶部拼音行的各段。
    pub(crate) preedit: Vec<(String, PreeditKind)>,

    /// 光标在拼音行里的字符位置。
    pub(crate) cursor: usize,

    /// 候选行（页内顺序；竖排倒着画时仍按这个顺序存，序号才不会跟着颠倒）。
    pub(crate) rows: Vec<Row>,

    /// 高亮行下标（页内）；`usize::MAX` 为不高亮。
    pub(crate) highlight: usize,

    /// 页码，只有多页时有。
    pub(crate) footer: Option<String>,

    /// 整句补全，画在拼音行右侧。
    pub(crate) sentence: Option<String>,

    /// 屏幕提示（删候选后的「已删除…」），画在拼音行下方。
    pub(crate) notice: Option<String>,

    /// 候选排布。
    pub(crate) layout: LayoutMode,

    /// 矩阵每行几格（`> 0` 是横排展开成矩阵）；单行为 0。
    pub(crate) columns: usize,

    /// 矩阵各列留几个候选字宽。
    pub(crate) column_ems: Vec<f32>,

    /// 竖排窗口在光标上方时倒着排（第一项贴着光标）。
    pub(crate) reversed: bool,
}

impl View {
    pub(crate) fn set(&mut self, frame: &Frame) {
        self.layout = frame.layout;
        self.reversed = false;
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
                let mut row = row::from_candidate(
                    if grid { i % frame.columns } else { i },
                    candidate,
                    frame.aux_code_show,
                    frame.translation_pos,
                    frame.english_reading_ipa,
                );
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
        self.sentence = frame.sentence.clone();
        self.notice = frame.notice.clone();
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

#[cfg(test)]
mod tests {
    use qingjian_core::{Candidate, CandidateKind, CandidateList};
    use qingjian_platform::PreeditMode;
    use qingjian_platform::protocol::{Frame, PreeditKind, PreeditSegment};

    use super::*;

    fn frame(mode: PreeditMode) -> Frame {
        Frame {
            preedit: vec![PreeditSegment {
                text: "ni'hao".to_owned(),
                kind: PreeditKind::Typed,
            }],
            preedit_mode: mode,
            ..Frame::default()
        }
    }

    fn candidate(text: &str) -> Candidate {
        Candidate {
            text: text.to_owned(),
            kind: CandidateKind::Chinese,
            syllables: Vec::new(),
            reading: None,
            translation: None,
            aux_code: None,
        }
    }

    /// 只有「只在行内」不给窗口拼音行；另两档窗口都得画。
    #[test]
    fn window_keeps_the_pinyin_row_unless_inline_only() {
        let mut view = View::default();
        view.set(&frame(PreeditMode::Both));
        assert_eq!(view.preedit.len(), 1);
        view.set(&frame(PreeditMode::Window));
        assert_eq!(view.preedit.len(), 1);
        view.set(&frame(PreeditMode::Inline));
        assert!(view.preedit.is_empty());
    }

    /// 矩阵只在页内顺序标序号（高亮那一行），空位与其余行都不标。
    #[test]
    fn matrix_labels_only_the_highlighted_row() {
        let frame = Frame {
            candidates: CandidateList {
                items: vec![
                    candidate("你"),
                    candidate("泥"),
                    candidate("拟"),
                    candidate("逆"),
                ],
            },
            columns: 2,
            highlight: 3,
            ..Frame::default()
        };
        let mut view = View::default();
        view.set(&frame);
        let indexes: Vec<&str> = view.rows.iter().map(|row| row.index.as_str()).collect();
        assert_eq!(indexes, ["", "", "1", "2"]);
    }

    /// 单行照旧从 1 标到 n。
    #[test]
    fn a_single_row_numbers_every_candidate() {
        let frame = Frame {
            candidates: CandidateList {
                items: vec![candidate("你"), candidate("泥")],
            },
            page_count: 1,
            ..Frame::default()
        };
        let mut view = View::default();
        view.set(&frame);
        let indexes: Vec<&str> = view.rows.iter().map(|row| row.index.as_str()).collect();
        assert_eq!(indexes, ["1", "2"]);
        assert!(view.footer.is_none());
    }

    /// 多页才画页码。
    #[test]
    fn footer_only_shows_on_multiple_pages() {
        let frame = Frame {
            page: 1,
            page_count: 3,
            ..Frame::default()
        };
        let mut view = View::default();
        view.set(&frame);
        assert_eq!(view.footer.as_deref(), Some("2/3"));
    }
}
