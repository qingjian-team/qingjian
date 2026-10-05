//! 组句的展示状态：缓冲变化时重查候选并重建 [`Composed`]，云端词异步并入，高亮 / 翻页，按状态生成给 DLL 的帧。

mod state;

use qingjian_core::{
    Candidate, CandidateKind, CandidateLayout, CandidateList, Cell, CloudWord, GRID_ROWS, Grid,
    Query,
};
use qingjian_platform::protocol::{Frame, PROTOCOL_VERSION, PreeditKind, PreeditSegment};

pub(super) use self::state::{Composed, TypedKeys};
use super::Router;

impl Router {
    /// 缓冲变化后：按 Engine 状态重建 [`Composed`]，发一次云联想请求，归零高亮与整句补全。
    pub(super) fn recompose(&mut self) {
        self.highlight = 0;
        self.navigated = false;
        self.grid = None;
        self.sentence = None;
        if self.engine.composition().is_empty() {
            self.composed = None;
            self.cancel_prediction();
            self.stop_rescoring();
            return;
        }
        self.attach_loaded_model();
        let built = self.engine.query().ok().map(|query| {
            let (preedit, cursor, typed_keys) = marked_parts(&query);
            (query.candidates.items.clone(), preedit, cursor, typed_keys)
        });
        self.composed = Some(match built {
            Some((items, preedit, cursor, typed_keys)) => {
                let layout =
                    CandidateLayout::new(items, self.config.page_size, self.config.cloud_slots);
                if self.engine.prediction_enabled() {
                    self.engine.request_prediction(None, layout.local());
                }
                Composed::Candidates {
                    preedit,
                    cursor,
                    typed_keys,
                    layout,
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
        if let Some(Composed::Candidates { layout, .. }) = self.composed.as_mut() {
            let words: Vec<Candidate> = prediction
                .words
                .into_iter()
                .map(CloudWord::into_candidate)
                .collect();
            layout.set_cloud(words);
            self.sentence = prediction.sentence;
        }
    }

    pub(super) fn cancel_prediction(&mut self) {
        if self.engine.prediction_enabled() {
            self.engine.cancel_prediction();
        }
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

    /// 矩阵里按阅读顺序移一格（越过行尾到下一行开头），跳过空位；没展开时不动。
    /// 照微信输入法的逻辑，`←` 顶在第一个候选上移不动时收回单行（先退回单行，再按一下才轮到拼音光标）。
    /// 返回是否动了（收回也算动）。
    pub(super) fn move_cells(&mut self, delta: isize) -> bool {
        let Some(Composed::Candidates { layout, .. }) = &self.composed else {
            return false;
        };
        let Some(mut grid) = self.grid else {
            return false;
        };
        let moved = grid.move_cells(layout, self.highlight, delta);
        self.grid = Some(grid);
        if moved.is_none() && delta < 0 {
            return self.collapse_grid();
        }
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

    /// 协议比 Server 老的 DLL 不认识 `AuxCode` 段，收到会整条消息解析失败；给它的码段降级成普通拼音段。
    fn downgrade_for_old_dll(&self, frame: &mut Frame) {
        if self.focused_dll_protocol() >= PROTOCOL_VERSION {
            return;
        }
        for segment in &mut frame.preedit {
            if segment.kind == PreeditKind::AuxCode {
                segment.kind = PreeditKind::Typed;
            }
        }
    }

    /// 焦点会话的 DLL 协议版本；没有焦点会话时按最新（不必降级）。
    fn focused_dll_protocol(&self) -> u32 {
        self.focused
            .and_then(|session| self.sessions.get(&session))
            .map_or(PROTOCOL_VERSION, |info| info.protocol)
    }

    fn raw_frame(&self) -> Frame {
        if let Some(translation) = &self.translation {
            return self.translation_frame(translation);
        }
        match &self.composed {
            None => Frame::default(),
            Some(Composed::Raw { text, cursor }) => Frame {
                preedit: vec![PreeditSegment {
                    text: text.clone(),
                    kind: PreeditKind::Typed,
                }],
                preedit_mode: self.config.preedit,
                cursor: *cursor,
                candidates: CandidateList { items: Vec::new() },
                highlight: usize::MAX,
                page: 0,
                page_count: 1,
                layout: self.config.layout,
                columns: 0,
                column_ems: Vec::new(),
                theme: self.config.theme,
                aux_code_show: self.config.aux_code_show,
                sentence: None,
                notice: self.notice.clone(),
            },
            Some(Composed::Candidates {
                preedit,
                cursor,
                layout,
                ..
            }) => {
                let page_size = self.config.page_size;
                let highlight = self.highlight.min(layout.len().saturating_sub(1));
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
                    preedit: preedit.clone(),
                    preedit_mode: self.config.preedit,
                    cursor: *cursor,
                    candidates,
                    highlight: highlight - first,
                    page,
                    page_count: layout.pages().max(1),
                    layout: self.config.layout,
                    columns,
                    column_ems: (columns > 0)
                        .then(|| Grid::column_ems(layout))
                        .unwrap_or_default(),
                    theme: self.config.theme,
                    aux_code_show: self.config.aux_code_show,
                    sentence: self.sentence.clone(),
                    notice: self.notice.clone(),
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
