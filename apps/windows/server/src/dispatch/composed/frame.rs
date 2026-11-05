//! 按组句状态生成帧：给 DLL 的（老协议降级、输入框显示原始按键）与自绘窗的，符号面板翻译组的 preedit 是译文。

use qingjian_core::candidate::arrows::Focus;
use qingjian_core::{Candidate, CandidateKind, CandidateList, Cell, Engine, Grid, Query};
use qingjian_platform::PreeditMode;
use qingjian_platform::protocol::{Frame, PROTOCOL_VERSION, PreeditKind, PreeditSegment};

use super::{Composed, marked_parts};
use crate::dispatch::Router;

impl Router {
    /// 按当前状态生成一帧：翻译评审优先；没在组句给空帧；否则给高亮所在的那一页。
    /// 焦点会话的 DLL 比 Server 老时按老协议降级（见 [`Self::downgrade_for_old_dll`]）。
    pub(in crate::dispatch) fn current_frame(&self) -> Frame {
        let mut frame = self.raw_frame();
        self.show_typed_keys(&mut frame);
        self.downgrade_for_old_dll(&mut frame);
        frame
    }

    /// 给 DLL 的帧只管应用输入框：双拼「输入框显示原始按键」开着时换成敲的键，拼音行由 Server 自绘照旧全拼。
    fn show_typed_keys(&self, frame: &mut Frame) {
        if self.translation.is_some() || self.engine.panel_translating() {
            return;
        }
        let Some(Composed::Candidates {
            typed_keys: Some(keys),
            ..
        }) = &self.composed
        else {
            return;
        };
        frame.preedit = vec![PreeditSegment {
            text: keys.text.clone(),
            kind: PreeditKind::Typed,
        }];
        frame.cursor = keys.cursor;
    }

    /// 自绘候选窗用的帧：不做老 DLL 降级，码段照常画。
    pub(in crate::dispatch) fn self_drawn_frame(&self) -> Frame {
        self.raw_frame()
    }

    /// 协议比 Server 老的 DLL 不认识 `AuxCode` 段与颜文字、符号两种候选，收到会整条消息解析失败：
    /// 码段降级成普通拼音段，那两种候选按 emoji 发（DLL 不看候选种类，只要解得出来）。
    fn downgrade_for_old_dll(&self, frame: &mut Frame) {
        if self.focused_dll_protocol() >= PROTOCOL_VERSION {
            return;
        }
        for segment in &mut frame.preedit {
            if segment.kind == PreeditKind::AuxCode {
                segment.kind = PreeditKind::Typed;
            }
        }
        for candidate in &mut frame.candidates.items {
            if matches!(
                candidate.kind,
                CandidateKind::Kaomoji | CandidateKind::Symbol
            ) {
                candidate.kind = CandidateKind::Emoji;
            }
        }
    }

    /// 焦点会话的 DLL 协议版本；没有焦点会话时按最新（不必降级）。
    pub(in crate::dispatch) fn focused_dll_protocol(&self) -> u32 {
        self.focused
            .and_then(|session| self.sessions.get(&session))
            .map_or(PROTOCOL_VERSION, |info| info.protocol)
    }

    fn raw_frame(&self) -> Frame {
        if let Some(translation) = &self.translation {
            return self.translation_frame(translation);
        }
        let mut frame = self.composed_frame();
        frame.symbol_panel = self.panel_open();
        if frame.symbol_panel && self.engine.panel_translating() {
            frame.panel_text_box = self.engine.composition().is_empty();
            self.translate_preedit(&mut frame);
        }
        frame
    }

    /// 翻译组：拼音画在面板的框里，应用里放最近一条译文；行内不显示拼音的设置也照样放，译文只能在那里看。
    fn translate_preedit(&self, frame: &mut Frame) {
        let text = self.engine.panel_preedit().unwrap_or_default().to_owned();
        frame.cursor = text.chars().count();
        frame.preedit = if text.is_empty() {
            Vec::new()
        } else {
            vec![PreeditSegment {
                text,
                kind: PreeditKind::Typed,
            }]
        };
        frame.preedit_mode = PreeditMode::Inline;
        frame.window_preedit = None;
    }

    fn composed_frame(&self) -> Frame {
        match &self.composed {
            None => Frame::default(),
            Some(Composed::Vietnamese { text, candidates }) => Frame {
                preedit: vec![PreeditSegment {
                    text: text.clone(),
                    kind: PreeditKind::Typed,
                }],
                preedit_mode: self.config.preedit,
                cursor: text.chars().count(),
                window_preedit: None,
                candidates: CandidateList {
                    items: candidates.clone(),
                },
                highlight: self.highlight.min(candidates.len().saturating_sub(1)),
                page: 0,
                page_count: 1,
                layout: self.config.layout,
                columns: 0,
                column_ems: Vec::new(),
                appearance: self.config.appearance,
                aux_code_show: false,
                translation_pos: false,
                english_reading_ipa: false,
                sentence: None,
                notice: None,
                symbol_panel: false,
                panel_text_box: false,
            },
            Some(Composed::Raw { text, cursor }) => Frame {
                preedit: vec![PreeditSegment {
                    text: text.clone(),
                    kind: PreeditKind::Typed,
                }],
                preedit_mode: self.config.preedit,
                cursor: *cursor,
                window_preedit: None,
                candidates: CandidateList { items: Vec::new() },
                highlight: usize::MAX,
                page: 0,
                page_count: 1,
                layout: self.config.layout,
                columns: 0,
                column_ems: Vec::new(),
                appearance: self.config.appearance,
                aux_code_show: self.config.aux_code_show,
                translation_pos: self.config.translation_pos,
                english_reading_ipa: self.config.show_english_pronunciation,
                sentence: None,
                notice: self.notice.clone(),
                symbol_panel: false,
                panel_text_box: false,
            },
            Some(Composed::Candidates {
                preedit,
                cursor,
                layout,
                spelling,
                ..
            }) => {
                let page_size = self.config.page_size;
                let highlight = self.highlight.min(layout.len().saturating_sub(1));
                // 高亮候选的读音切法与首选不同（`ganga` 的 尴尬 / 刚啊）：拼音行按它重切
                let candidate = layout.candidate(highlight);
                let followed = (spelling.spelling_of(candidate) != spelling.spelled).then(|| {
                    let mut followed = (**spelling).clone();
                    followed.follow(candidate);
                    followed
                });
                let query = followed.as_ref().unwrap_or(spelling);
                let (preedit, cursor) = match &followed {
                    Some(followed) => {
                        let (preedit, cursor, _) = marked_parts(followed);
                        (preedit, cursor)
                    }
                    None => (preedit.clone(), *cursor),
                };
                let window_preedit = window_parts(&self.engine, query, candidate, &preedit);
                let page = highlight / page_size;
                // 横排展开成矩阵时画视口里的几行（空位是占位候选），序号只标在高亮所在那一行；
                // 单行时只画当前页。高亮下标跟着换成视口内的。
                let viewport = self.grid.as_ref().map(|grid| grid.rows(layout));
                // 视口首格在整份候选里的下标（单行时是页首）
                let first = match &viewport {
                    Some(rows) => rows.start * page_size,
                    None => page * page_size,
                };
                let (items, columns) =
                    match viewport {
                        Some(rows) => {
                            let mut items = Vec::new();
                            for row in rows {
                                let mut cells = layout.page(row);
                                cells.resize(page_size, Cell::Empty);
                                items.extend(cells.into_iter().map(|cell| {
                                    cell.candidate().cloned().unwrap_or_else(empty_cell)
                                }));
                            }
                            (items, page_size)
                        }
                        None => (
                            layout
                                .page(page)
                                .into_iter()
                                .filter_map(|cell| cell.candidate().cloned())
                                .collect(),
                            0,
                        ),
                    };
                let mut candidates = CandidateList { items };
                self.engine.annotate(&mut candidates);
                Frame {
                    preedit,
                    preedit_mode: self.config.preedit,
                    cursor,
                    window_preedit,
                    candidates,
                    // 方向键在拼音行里时候选不画高亮，空格照样上屏原来高亮的那个
                    highlight: if self.config.two_area_arrows() && self.focus == Focus::Pinyin {
                        usize::MAX
                    } else {
                        highlight - first
                    },
                    page,
                    page_count: layout.pages().max(1),
                    layout: self.config.layout,
                    columns,
                    column_ems: if columns > 0 {
                        Grid::column_ems(layout)
                    } else {
                        Vec::new()
                    },
                    appearance: self.config.appearance,
                    aux_code_show: self.config.aux_code_show,
                    translation_pos: self.config.translation_pos,
                    english_reading_ipa: self.config.show_english_pronunciation,
                    sentence: self.sentence.clone(),
                    notice: self.notice.clone(),
                    symbol_panel: false,
                    panel_text_box: false,
                }
            }
        }
    }
}

/// 矩阵视口里空位的占位候选：文本为空，渲染端按空格子画。
fn empty_cell() -> Candidate {
    Candidate {
        text: String::new(),
        kind: CandidateKind::Chinese,
        syllables: Vec::new(),
        reading: None,
        translation: None,
        aux_code: None,
    }
}

/// 候选窗口顶部的拼音行：双拼「候选窗显示按键」开着时是敲的键；带调拼音开着、高亮候选读得出声调时换成它的读音；
/// 与行内拼音相同时不另给。
fn window_parts(
    engine: &Engine,
    query: &Query,
    candidate: Option<&Candidate>,
    preedit: &[PreeditSegment],
) -> Option<Box<(Vec<PreeditSegment>, usize)>> {
    let (segments, cursor) = if query.shuangpin_raw_window {
        query.window_segments(None)
    } else {
        let toned = engine.toned_syllables(candidate?)?;
        query.window_segments(Some(&toned))
    };
    let segments: Vec<PreeditSegment> = segments.iter().map(Into::into).collect();
    (segments != preedit).then(|| Box::new((segments, cursor)))
}
