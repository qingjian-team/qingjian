//! 查询与呈现：刷新候选、画候选窗、整句补全、翻页与高亮。

use super::*;
use qingjian_core::candidate::arrows::{self, Arrow, ArrowAction};
use qingjian_platform::HighlightKeys;

/// 拼音只在候选窗口显示时，组句中交给应用的 marked text。
const HIDDEN_MARKED_TEXT: &str = "\u{200B}";

impl QingjianInputController {
    /// 按当前缓冲区重新查候选、更新 marked text，回到第一页并重画候选窗口。
    pub(super) fn refresh(&self, client: TextClient<'_>) {
        // 本地整句模型、选词模型与自动空格要看光标前文：一段组句只在第一键读一次（组句中它不变；应用偶尔不回话也不至于让前文来回换），
        // 读应用文本要等应用回话，放在借 Host 之外（见 request_prediction）
        // 模型还在后台加载也读：接上时会补这一轮的重排，前文得先备好
        let wants_context = host::with(|h| {
            h.attach_loaded_model();
            (h.engine.has_sentence_scorer()
                || h.engine.has_choice_model()
                || h.model_loading()
                || h.engine.auto_space())
                && h.engine.composition().text().chars().count() == 1
        })
        .unwrap_or(false);
        let before = if wants_context && !secure_input::enabled() {
            Some(
                client
                    .surrounding_text(RESCORE_LOOKBACK, 0)
                    .map(|text| text.before),
            )
        } else {
            None
        };
        let Some((marked, cursor, inline)) = host::with(|h| {
            if let Some(before) = before {
                h.engine.set_rescoring_context(before);
            }
            // 查询失败（整段切不动）时退回显示原始字母
            let mut marked = h.engine.composition().text().to_owned();
            let mut cursor = h.engine.composition().cursor();
            let mut preedit = Preedit::plain(&marked, cursor);
            let candidates = h
                .engine
                .query()
                .map(|mut query| {
                    h.engine.annotate(&mut query.candidates);
                    marked = query.marked_text();
                    cursor = query.marked_cursor();
                    preedit =
                        Preedit::from_marked(&query.marked_segments(), query.segments_cursor());
                    let candidates = std::mem::take(&mut query.candidates.items);
                    (candidates, Some(query))
                })
                .unwrap_or_default();
            let (candidates, spelling) = candidates;
            h.reset_session(preedit, candidates);
            h.session.spelling = spelling;
            h.schedule_rescoring();
            (marked, cursor, h.preedit_mode.inline())
        }) else {
            return;
        };
        // 配置成只在候选窗口显示拼音时，组句中放一个零宽空格当 marked text：微信 4.x 这类应用看不到 marked text
        // 就当输入法没在组句，自己再把字母、退格处理一遍（#815 #816）
        if inline {
            client.set_marked_text(&marked, cursor);
        } else if marked.is_empty() {
            client.set_marked_text("", 0);
        } else {
            client.set_marked_text(HIDDEN_MARKED_TEXT, 0);
        }
        // 先发联想再画：发出去就留好云端槽位，画出来的第一帧本地候选就已经在最终位置
        if !marked.is_empty() {
            let candidates = host::with(|h| h.session.layout.local().to_vec()).unwrap_or_default();
            self.request_prediction(client, &candidates, false);
        }
        self.render(client);
    }

    /// 缓冲区里只有一个 `?` 而用户按了别的键：把它还原成问号上屏（中文遵循标点设置、英文半角）、清空缓冲区。
    /// 返回是否发生了还原。
    pub(super) fn restore_bare_question(&self, client: TextClient<'_>) -> bool {
        let english = host::with(|h| h.english_mode()).unwrap_or(false);
        let restored = host::with(|h| {
            let mark = h.engine.restore_bare_question(english)?;
            h.cancel_prediction();
            Some(mark)
        })
        .flatten();
        let Some(mark) = restored else {
            return false;
        };
        client.insert_text(&mark);
        self.refresh(client);
        true
    }

    /// 记下光标位置并按会话状态重画候选窗口；高亮换了切法不同的候选时 marked text 跟着换。
    pub(super) fn render(&self, client: TextClient<'_>) {
        let anchor = client.caret_rect();
        let marked = host::with(|h| {
            let marked = h.follow_highlight().filter(|_| h.preedit_mode.inline());
            h.anchor = anchor;
            h.render();
            marked
        })
        .flatten();
        if let Some((text, cursor)) = marked {
            client.set_marked_text(&text, cursor);
        }
    }

    /// 发一次联想请求。Secure Input 里绝不发；没接联想器时是空操作。
    /// `manual` 是组句中按了按需联想键：不等停键，`[predict] automatic` 关着也发。
    ///
    /// 读上下文要等应用回话，这段时间 IMK 可能把 `deactivateServer:` 之类的回调插进来，
    /// 所以分两次借 Host：先拿策略、放开借用去读、再借回来发请求。
    pub(super) fn request_prediction(
        &self,
        client: TextClient<'_>,
        candidates: &[Candidate],
        manual: bool,
    ) {
        let policy = host::with(|h| {
            if !h.engine.prediction_enabled() {
                return None;
            }
            if secure_input::enabled() {
                tracing::debug!("Secure Input 中，不联想");
                h.cancel_prediction();
                return None;
            }
            Some(h.engine.prediction_policy())
        })
        .flatten();
        let Some(policy) = policy else {
            return;
        };
        let surrounding = client.surrounding_text(policy.before, policy.after);
        host::with(|h| {
            tracing::debug!(
                has_context = surrounding.is_some(),
                pinyin = h.engine.composition().scope(),
                "联想请求"
            );
            let sequence = if manual {
                h.engine.request_manual_prediction(surrounding, candidates)
            } else {
                h.engine.request_prediction(surrounding, candidates)
            };
            match sequence {
                Some(sequence) => h.await_prediction(manual.then_some(sequence)),
                None => h.cancel_prediction(),
            }
        });
    }

    /// 接受组句中的整句补全：作用域内的拼音作废，句子上屏。没有补全返回 false。
    pub(super) fn accept_sentence(&self, client: TextClient<'_>) -> bool {
        let Some(text) = host::with(|h| h.sentence.take()).flatten() else {
            return false;
        };
        host::with(|h| h.engine.accept_prediction(&text));
        tracing::debug!(%text, "接受整句补全");
        client.insert_text(&text);
        self.refresh(client);
        true
    }

    /// 方向键分两区时（[`Host::two_area_arrows`]）照 Core 的 [`arrows::route`] 走，规则与 Windows 壳的 `two_area_key` 同一份。
    /// 返回是不是归了这套规则（不是的话调用方照旧走 [`Self::move_highlight`] / [`Self::move_cells`]）。
    pub(super) fn two_area_arrow(&self, arrow: Arrow, client: TextClient<'_>) -> bool {
        /// 拼音光标动了要重新查询；高亮或焦点动了只重画。
        enum Moved {
            Cursor,
            Highlight(bool),
        }
        let moved = host::with(|h| {
            if !h.two_area_arrows() || h.session.layout.is_empty() {
                return None;
            }
            let matrix = h.session.matrix(h.grid_keys());
            Some(match arrows::route(arrow, h.session.focus, matrix) {
                ArrowAction::MoveCursor(delta) => {
                    if delta < 0 {
                        h.engine.move_cursor_left();
                    } else {
                        h.engine.move_cursor_right();
                    }
                    Moved::Cursor
                }
                ArrowAction::MoveHighlight(delta) if h.session.grid.is_some() => {
                    Moved::Highlight(h.session.move_cells(delta))
                }
                ArrowAction::MoveHighlight(delta) => {
                    Moved::Highlight(h.session.move_highlight(delta))
                }
                ArrowAction::MoveRow(delta) => Moved::Highlight(h.session.move_rows(delta)),
                ArrowAction::Expand => Moved::Highlight(h.session.move_rows(1)),
                ArrowAction::Focus(focus) => {
                    h.session.focus = focus;
                    Moved::Highlight(true)
                }
                ArrowAction::Stay => Moved::Highlight(false),
            })
        })
        .flatten();
        match moved {
            Some(Moved::Cursor) => self.refresh(client),
            Some(Moved::Highlight(true)) => self.render(client),
            Some(Moved::Highlight(false)) | None => {}
        }
        moved.is_some()
    }

    /// 上下键。高亮逐个移动，越过页边自动翻页；横排矩阵开着（`[general] horizontal_grid`）时改为：
    /// 单行先展开成矩阵，在矩阵里换行（视口跟着滚）。高亮移动键选了 `←` / `→` 时上下键不动（键照样吃掉，不漏给应用挪插入符）。
    pub(super) fn move_highlight(&self, delta: isize, client: TextClient<'_>) {
        let changed = host::with(|h| {
            if h.grid_keys() {
                let moved = h.session.move_rows(delta);
                // ↑ 顶在第一排再往上：回到第一个候选（不是毫无动作）
                if !moved && delta < 0 {
                    h.session.jump_to_first()
                } else {
                    moved
                }
            } else if h.highlight_keys == HighlightKeys::UpDown {
                h.session.move_highlight(delta)
            } else {
                false
            }
        })
        .unwrap_or(false);
        if changed {
            self.render(client);
        }
    }

    /// 左右键。横排矩阵开着、或高亮移动键选了 `←` / `→`，而且有候选时归候选：单行里逐个移动高亮（到页边自动翻页，不展开），展开后按阅读顺序跨行移动——
    /// 上下键被矩阵占了，单行里只剩左右键能挪高亮。规则与 Windows 壳的 `horizontal_key` 一致（把「拼音行 + 候选栏」当成
    /// 一条从左到右的序列走）：`→` 先走光标、到末位溢出成选词；`←` 先退候选、顶在第一个候选上回拼音行。
    /// 返回是不是归了候选（不是的话调用方照旧移动拼音光标）。
    pub(super) fn move_cells(&self, delta: isize, client: TextClient<'_>) -> bool {
        let handled = host::with(|h| {
            if !h.arrows_move_cells() || h.session.layout.is_empty() {
                return None;
            }
            // 拼音光标已经离开末尾（正在改拼音）：→ 继续归光标
            if delta > 0 {
                let composition = h.engine.composition();
                if composition.cursor() < composition.text().len() {
                    return None;
                }
            }
            // 顶在第一个候选上：交还拼音光标（去改已输入的拼音）
            if delta < 0 && h.session.highlighted == 0 {
                return None;
            }
            Some(if h.session.grid.is_some() {
                h.session.move_cells(delta)
            } else {
                h.session.move_highlight(delta)
            })
        })
        .flatten();
        if handled == Some(true) {
            self.render(client);
        }
        handled.is_some()
    }

    /// 每个键都问一次应用标识（activateServer 时进程刚拉起可能还拿不到），变了才告诉 Engine。
    pub(super) fn note_application(&self, client: &TextClient<'_>) {
        let app = client.bundle_identifier();
        host::with(|h| {
            if h.engine.application() != app.as_deref() {
                h.engine.set_application(app);
            }
        });
    }

    /// 翻页，高亮落到新页第一项。已在首页 / 末页时不动。
    pub(super) fn turn_page(&self, delta: isize, client: TextClient<'_>) -> bool {
        let turned = host::with(|h| {
            // 矩阵展开着：翻页键一次翻一屏
            let turned = if h.session.grid.is_some() {
                h.session.move_screens(delta)
            } else {
                h.session.turn_page(delta)
            };
            if turned {
                h.engine.note_page_turn();
            }
            turned
        })
        .unwrap_or(false);
        if turned {
            self.render(client);
        }
        true
    }

    /// 这个字符是不是翻页键：-1 上一页、1 下一页。先认主翻页键，再认勾上的额外翻页键（`[shortcut] extra_page_keys`），与 Windows 一致。
    pub(super) fn page_step(c: char) -> Option<isize> {
        host::with(|h| {
            if c == h.page_keys.0 {
                Some(-1)
            } else if c == h.page_keys.1 {
                Some(1)
            } else {
                h.extra_page_keys.step(c)
            }
        })
        .flatten()
    }

    /// 翻页键：翻得动就翻页并吃键，返回 `true`。`-` / `=` 在**没有上一页 / 下一页**时返回 `false`，
    /// 由调用方按普通字符处理 —— 没有上一页时 `-` 就是 `-`（见 `docs/user/personalize/keys-settings.md`
    /// 的 `[general] page_keys`）。`fall_through` 为假时（问字模式里按键是问题内容）翻不动也吃键；
    /// 别的翻页键（`[` `]` `,` `.`）一律吃键；`step` 是 [`Self::page_step`] 的结果，`None` 时不是翻页键 —— 落字会往文档里插标点。非翻页键一律返回 `false`。
    pub(super) fn page_key(
        &self,
        c: char,
        step: Option<isize>,
        fall_through: bool,
        client: TextClient<'_>,
    ) -> bool {
        let Some(step) = step else {
            return false;
        };
        // 矩阵展开着时翻页键翻的是整屏，能不能翻交给 `turn_page` 试，`-` / `=` 不在这种情形下落字
        let (single_row, can) =
            host::with(|h| (h.session.grid.is_none(), h.session.can_turn_page(step)))
                .unwrap_or((false, false));
        if fall_through && single_row && !can && matches!(c, '-' | '=') {
            return false;
        }
        self.turn_page(step, client);
        true
    }
}
