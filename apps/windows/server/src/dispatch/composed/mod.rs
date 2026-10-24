//! 组句的展示状态：缓冲变化时重查候选并重建 [`Composed`]，云端词异步并入，高亮 / 翻页，按状态生成给 DLL 的帧。

mod sentence_translation;
mod state;

use qingjian_core::candidate::arrows::{Focus, Matrix};
use qingjian_core::{
    Candidate, CandidateKind, CandidateLayout, CandidateList, Cell, CloudWord, Engine, GRID_ROWS,
    Grid, Language, Query, Sense, SurroundingText, Translation,
};
use qingjian_platform::protocol::{Frame, PROTOCOL_VERSION, PreeditKind, PreeditSegment};

use self::sentence_translation::SentenceTranslation;
pub(super) use self::state::{Composed, TypedKeys};
use super::Router;

impl Router {
    /// 缓冲变化后：按 Engine 状态重建 [`Composed`]，发一次云联想请求，归零高亮与整句补全。
    pub(super) fn recompose(&mut self) {
        let previous_sentence_translation =
            self.composed.take().and_then(|composed| match composed {
                Composed::Candidates {
                    sentence_translation,
                    ..
                } => sentence_translation,
                Composed::Raw { .. } => None,
            });
        self.highlight = 0;
        self.navigated = false;
        self.grid = None;
        self.sentence = None;
        if self.engine.composition().is_empty() {
            self.focus = Focus::default();
            // 清空时也要撤掉 worker 中的待发请求，避免无用请求占掉最短间隔。
            self.engine.cancel_prediction();
            self.composed = None;
            self.stop_rescoring();
            return;
        }
        self.engine.cancel_composition_prediction();
        self.attach_loaded_model();
        let built = self.engine.query().ok().map(|mut query| {
            let (preedit, cursor, typed_keys) = marked_parts(&query);
            let items = std::mem::take(&mut query.candidates.items);
            (items, preedit, cursor, typed_keys, Box::new(query))
        });
        self.composed = Some(match built {
            Some((items, preedit, cursor, typed_keys, spelling)) => {
                let layout =
                    CandidateLayout::new(items, self.config.page_size, self.config.cloud_slots);
                if self.config.cloud_enabled && self.engine.prediction_enabled() {
                    let surrounding = self.surrounding_for_prediction();
                    self.engine.request_prediction(surrounding, layout.local());
                }
                let sentence_candidate = (self.config.sentence_translation
                    && self.engine.learning_language() != Language::Chinese)
                    .then(|| {
                        layout
                            .local()
                            .iter()
                            .find(|candidate| candidate.kind == CandidateKind::Sentence)
                    })
                    .flatten();
                let sentence_translation = sentence_candidate.and_then(|candidate| {
                    if let Some(annotation) = previous_sentence_translation
                        && annotation.text == candidate.text
                    {
                        return Some(annotation);
                    }
                    self.engine.cancel_sentence_translation();
                    self.engine
                        .request_sentence_translation(&candidate.text)
                        .map(|sequence| {
                            SentenceTranslation::pending(candidate.text.clone(), sequence)
                        })
                });
                if sentence_translation.is_none() {
                    self.engine.cancel_sentence_translation();
                }
                Composed::Candidates {
                    preedit,
                    cursor,
                    typed_keys,
                    layout,
                    spelling,
                    sentence_translation,
                    manual_sequence: None,
                }
            }
            None => {
                self.cancel_prediction();
                let composition = self.engine.composition();
                let text = composition.text().to_owned();
                let cursor = text[..composition.cursor()].chars().count();
                Composed::Raw { text, cursor }
            }
        });
        self.schedule_rescoring();
    }

    /// 拉一次云联想结果：云端词并进候选布局，整句补全记下；翻译评审时结果是译文。
    pub(super) fn poll_prediction(&mut self) {
        if !self.engine.prediction_enabled() {
            return;
        }
        let Some(prediction) = self.engine.poll_prediction() else {
            return;
        };
        if self
            .candidate_translation
            .as_ref()
            .is_some_and(|(sequence, _)| *sequence == prediction.sequence)
        {
            let (_, candidate) = self.candidate_translation.take().expect("刚检查过候选译文");
            let updated = prediction
                .sentence
                .is_some_and(|text| self.engine.set_candidate_translation(&candidate, &text));
            self.notice = Some(if updated {
                self.refresh_candidate_translation(&candidate.text);
                format!("已更新「{}」的译文", candidate.text)
            } else {
                "模型没有生成可用译文".to_owned()
            });
            return;
        }
        if self.translation.is_some() {
            match prediction.sentence {
                Some(text) => {
                    if let Some(translation) = self.translation.as_mut() {
                        translation.result = Some(text);
                    }
                }
                None => {
                    tracing::info!("翻译选中文字：云端没有给出译文");
                    self.translation = None;
                }
            }
            return;
        }
        let is_sentence_translation = matches!(
            &self.composed,
            Some(Composed::Candidates {
                sentence_translation: Some(annotation),
                ..
            }) if annotation.sequence == prediction.sequence
        );
        if is_sentence_translation {
            if let Some(Composed::Candidates {
                sentence_translation: Some(annotation),
                ..
            }) = self.composed.as_mut()
            {
                annotation.translation = prediction.sentence.map(|text| {
                    Translation::new(
                        self.engine.learning_language(),
                        vec![Sense {
                            part_of_speech: None,
                            text,
                            reading: None,
                            fresh: false,
                        }],
                    )
                });
            }
            return;
        }
        if let Some(Composed::Candidates {
            layout,
            manual_sequence,
            ..
        }) = self.composed.as_mut()
        {
            let manual = manual_sequence.take() == Some(prediction.sequence);
            self.sentence = prediction.sentence;
            if self.navigated || self.highlight >= self.config.page_size {
                return;
            }
            let words = prediction
                .words
                .into_iter()
                .map(CloudWord::into_candidate)
                .collect();
            if manual {
                layout.set_cloud_first(words);
            } else {
                layout.set_cloud(words);
            }
        }
    }

    pub(super) fn cancel_prediction(&mut self) {
        if let Some(Composed::Candidates {
            manual_sequence, ..
        }) = self.composed.as_mut()
        {
            *manual_sequence = None;
        }
        if self.engine.prediction_enabled() {
            self.engine.cancel_prediction();
        }
    }

    /// DLL 送来新的光标前文后重发一次云联想：作废在飞的空上下文请求，让云端拿得到 before。
    pub(super) fn refresh_prediction(&mut self) {
        if !self.config.cloud_enabled {
            return;
        }
        let Some(Composed::Candidates {
            layout,
            manual_sequence,
            ..
        }) = &self.composed
        else {
            return;
        };
        if manual_sequence.is_some() {
            self.request_manual_prediction();
            return;
        }
        // 手动结果已到后保持当前布局；新输入会重建状态并恢复自动联想。
        if layout.cloud_first() {
            return;
        }
        let local = layout.local().to_vec();
        let surrounding = self.surrounding_for_prediction();
        self.engine.request_prediction(surrounding, &local);
    }

    /// 给当前的本地整句候选补上已经返回的云端译文。
    ///
    /// 候选布局保留查询原样；画帧、点击和译词快捷键各自拿副本时统一补这一层，
    /// 才不会让同一条译文只有显示、不能上屏。
    pub(super) fn annotate_sentence_translation(&self, candidates: &mut CandidateList) {
        let Some(Composed::Candidates {
            sentence_translation: Some(annotation),
            ..
        }) = &self.composed
        else {
            return;
        };
        let Some(translation) = &annotation.translation else {
            return;
        };
        for candidate in &mut candidates.items {
            if candidate.kind == CandidateKind::Sentence && candidate.text == annotation.text {
                candidate.translation = Some(translation.clone());
            }
        }
    }

    /// 显式触发时使用同一份应用上下文，并记录序号供结果布局匹配。
    pub(super) fn request_manual_prediction(&mut self) -> bool {
        if !self.config.cloud_enabled {
            return false;
        }
        let surrounding = self.surrounding_for_prediction();
        let Some(Composed::Candidates {
            layout,
            manual_sequence,
            ..
        }) = self.composed.as_mut()
        else {
            return false;
        };
        *manual_sequence = self
            .engine
            .request_manual_prediction(surrounding, layout.local());
        manual_sequence.is_some()
    }

    /// DLL 报来的光标前文包成云联想要的形状；没有就 `None`（Engine 只靠拼音猜）。
    pub(super) fn surrounding_for_prediction(&self) -> Option<SurroundingText> {
        (!self.surrounding_before.is_empty()).then(|| SurroundingText {
            before: self.surrounding_before.clone(),
            after: String::new(),
        })
    }

    /// 高亮移动 `delta`，夹在 `[0, 末尾]`，到页边自然换页。
    pub(super) fn move_highlight(&mut self, delta: isize) {
        let count = self.candidate_count();
        if count == 0 {
            self.highlight = 0;
            return;
        }
        let next = (self.highlight as isize + delta).clamp(0, count as isize - 1) as usize;
        self.navigated |= next != self.highlight;
        self.highlight = next;
    }

    /// 整页翻 `step`，高亮落到目标页第一个候选；矩阵展开着时一次翻一屏（与 macOS 壳的 `turn_page` 一致）。
    pub(super) fn page(&mut self, step: isize) {
        if self.grid.is_some() {
            if self.move_screens(step) {
                self.engine.note_page_turn();
            }
            return;
        }
        let count = self.candidate_count();
        if count == 0 {
            self.highlight = 0;
            return;
        }
        let page_size = self.config.page_size;
        let page_count = count.div_ceil(page_size);
        let current = (self.highlight / page_size) as isize;
        let target = (current + step).clamp(0, page_count as isize - 1) as usize;
        if target != current as usize {
            self.navigated = true;
            self.engine.note_page_turn();
        }
        self.highlight = (target * page_size).min(count - 1);
    }

    /// 往 `step` 方向还有没有页可翻。
    pub(super) fn can_page(&self, step: isize) -> bool {
        let page_count = self.candidate_count().div_ceil(self.config.page_size) as isize;
        let target = (self.highlight / self.config.page_size) as isize + step;
        (0..page_count).contains(&target)
    }

    /// 横排矩阵：上 / 下键竖着移 `delta` 行（列不变，目标格没候选就取最近的）；还是单行就先从当前页展开
    /// （展开本身算变化）。返回高亮是否真的动了。
    pub(super) fn move_rows(&mut self, delta: isize) -> bool {
        let expanding = self.grid.is_none();
        self.shift_grid_rows(delta) || expanding
    }

    /// 矩阵里顶在第一排还往上：高亮回到第一个候选、视口滚回顶部。返回是否动了。
    pub(super) fn jump_to_first(&mut self) -> bool {
        let Some(Composed::Candidates { layout, .. }) = &self.composed else {
            return false;
        };
        if self.grid.is_none() || self.highlight == 0 {
            return false;
        }
        self.highlight = 0;
        self.grid = Some(Grid::at(layout, 0));
        self.navigated = true;
        true
    }

    /// 给 `arrows::route` 的矩阵状态：没开「横排时展开成多行」、单行、展开后高亮在不在第一行。
    pub(super) fn matrix(&self) -> Matrix {
        match self.grid {
            Some(_) => Matrix::Open {
                first_row: self.highlight < self.config.page_size,
            },
            None if self.config.grid_keys() => Matrix::Closed,
            None => Matrix::Off,
        }
    }

    /// 矩阵里按阅读顺序移一格（越过行尾到下一行开头），跳过空位；没展开时不动。返回是否动了。
    pub(super) fn move_cells(&mut self, delta: isize) -> bool {
        let Some(Composed::Candidates { layout, .. }) = &self.composed else {
            return false;
        };
        let Some(mut grid) = self.grid else {
            return false;
        };
        let moved = grid.move_cells(layout, self.highlight, delta);
        self.grid = Some(grid);
        self.land(moved)
    }

    /// 矩阵里整屏翻（翻页键）：一次 [`GRID_ROWS`] 行。返回是否动了。
    pub(super) fn move_screens(&mut self, delta: isize) -> bool {
        self.shift_grid_rows(delta * GRID_ROWS as isize)
    }

    /// 收回单行（Esc 第一下），高亮留在原处。返回原来是不是展开着。
    pub(super) fn collapse_grid(&mut self) -> bool {
        self.grid.take().is_some()
    }

    /// 矩阵视口里竖着移 `delta` 行；没展开就从当前页展开。返回高亮是否真的动了。
    fn shift_grid_rows(&mut self, delta: isize) -> bool {
        let Some(Composed::Candidates { layout, .. }) = &self.composed else {
            return false;
        };
        let page = self.highlight / self.config.page_size;
        let mut grid = self.grid.take().unwrap_or_else(|| Grid::at(layout, page));
        let moved = grid.move_rows(layout, self.highlight, delta);
        self.grid = Some(grid);
        self.land(moved)
    }

    /// 高亮落到 `index`（`None` 是没动）。返回是否动了。
    fn land(&mut self, index: Option<usize>) -> bool {
        let Some(index) = index else {
            return false;
        };
        self.highlight = index;
        self.navigated = true;
        true
    }

    pub(super) fn candidate_count(&self) -> usize {
        match &self.composed {
            Some(Composed::Candidates { layout, .. }) => layout.len(),
            _ => 0,
        }
    }

    /// 候选布局里第 `index` 个（跨页下标）。
    pub(super) fn layout_candidate(&self, index: usize) -> Option<Candidate> {
        match &self.composed {
            Some(Composed::Candidates { layout, .. }) => layout.candidate(index).cloned(),
            _ => None,
        }
    }

    pub(super) fn commit_index(&mut self, index: usize) -> Option<String> {
        let candidate = self.layout_candidate(index)?;
        Some(self.engine.commit(&candidate))
    }

    /// 按当前状态生成一帧：翻译评审优先；没在组句给空帧；否则给高亮所在的那一页。
    /// 焦点会话的 DLL 比 Server 老时按老协议降级（见 [`Self::downgrade_for_old_dll`]）。
    pub(super) fn current_frame(&self) -> Frame {
        let mut frame = self.raw_frame();
        self.show_typed_keys(&mut frame);
        self.downgrade_for_old_dll(&mut frame);
        frame
    }

    /// 给 DLL 的帧只管应用输入框：双拼「输入框显示原始按键」开着时换成敲的键，拼音行由 Server 自绘照旧全拼。
    fn show_typed_keys(&self, frame: &mut Frame) {
        if self.translation.is_some() {
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
    pub(super) fn self_drawn_frame(&self) -> Frame {
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
    pub(super) fn focused_dll_protocol(&self) -> u32 {
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
        frame
    }

    fn composed_frame(&self) -> Frame {
        match &self.composed {
            None => Frame::default(),
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
                sentence: None,
                notice: self.notice.clone(),
                symbol_panel: false,
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
                self.annotate_sentence_translation(&mut candidates);
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
                    sentence: self.sentence.clone(),
                    notice: self.notice.clone(),
                    symbol_panel: false,
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

/// 候选窗口顶部的拼音行：带调拼音开着、高亮候选读得出声调时换成它的读音，与行内拼音相同时不另给。
fn window_parts(
    engine: &Engine,
    query: &Query,
    candidate: Option<&Candidate>,
    preedit: &[PreeditSegment],
) -> Option<Box<(Vec<PreeditSegment>, usize)>> {
    let toned = engine.toned_syllables(candidate?)?;
    let (segments, cursor) = query.window_segments(Some(&toned));
    let segments: Vec<PreeditSegment> = segments.iter().map(Into::into).collect();
    (segments != preedit).then(|| Box::new((segments, cursor)))
}

/// 一次查询的拼音行分段与光标，以及输入框另显示原始按键时的那一串。
/// 光标用 Core 的映射：自动补的 `'` 会让显示串比敲的长。
pub(super) fn marked_parts(query: &Query) -> (Vec<PreeditSegment>, usize, Option<TypedKeys>) {
    let preedit = query.marked_segments().iter().map(Into::into).collect();
    let typed_keys = query.shuangpin_raw_preedit.then(|| TypedKeys {
        text: query.marked_text(),
        cursor: query.marked_cursor(),
    });
    (preedit, query.segments_cursor(), typed_keys)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::{Router, RouterConfig};
    use qingjian_core::Engine;
    use qingjian_dictionary::Dictionary;
    use qingjian_platform::protocol::{ClientMessage, KeyEvent, KeyModifiers, SessionId};

    fn router() -> Router {
        let mut router = Router::new(
            Engine::new(Dictionary::parse("你\tni\t100\n").unwrap()),
            RouterConfig::default(),
        );
        router.handle(ClientMessage::OpenSession {
            session: SessionId(1),
            app: None,
            protocol: PROTOCOL_VERSION,
        });
        router
    }

    fn compose(router: &mut Router, text: &str) {
        for c in text.chars() {
            router
                .handle(ClientMessage::Key {
                    session: SessionId(1),
                    event: KeyEvent::new(c as u32, Some(c), KeyModifiers::default()),
                })
                .unwrap();
        }
    }

    /// 回归 #283：DLL 报来的光标前文要成为云联想的 `before` 上下文；
    /// 没前文时不带上下文，空文本不算上下文，组句结束即作废。
    #[test]
    fn surrounding_context_flows_to_prediction_and_clears_on_reset() {
        let mut router = router();
        compose(&mut router, "ni");
        assert!(router.surrounding_for_prediction().is_none());

        router.handle(ClientMessage::Surrounding {
            session: SessionId(1),
            text: "这个文件权限有问题，".to_owned(),
        });
        let surrounding = router.surrounding_for_prediction().expect("应带上前文");
        assert_eq!(surrounding.before, "这个文件权限有问题，");
        assert_eq!(surrounding.after, "");

        // 空文本不算上下文（Engine 只靠拼音猜，不猜错的）
        router.handle(ClientMessage::Surrounding {
            session: SessionId(1),
            text: String::new(),
        });
        assert!(router.surrounding_for_prediction().is_none());

        // 组句结束：前文作废，等下一段组句 DLL 再送
        router.handle(ClientMessage::Surrounding {
            session: SessionId(1),
            text: "别的内容".to_owned(),
        });
        router.stop_rescoring();
        assert!(router.surrounding_for_prediction().is_none());
    }

    /// 不是聚焦会话的前文不收，防止换应用后把 A 应用的文字带进 B 的联想。
    #[test]
    fn surrounding_from_unfocused_session_is_dropped() {
        let mut router = router();
        compose(&mut router, "ni");
        router.handle(ClientMessage::Surrounding {
            session: SessionId(99),
            text: "别的应用的前文".to_owned(),
        });
        assert!(router.surrounding_for_prediction().is_none());
    }
}
