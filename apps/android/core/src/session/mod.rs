//! 一个输入法服务实例对应的输入会话：结构与重画在这里，操作按职责分在子模块。

mod clipboard;
mod cloud;
mod contacts;
mod dictionaries;
mod display;
mod follow_up;
mod keys;
mod options;
mod prediction;
mod scheme;
mod swipe;
mod voice;

use std::path::PathBuf;
use std::sync::OnceLock;

use qingjian_core::{
    CandidateList, Engine, NoTranscriber, RomajiComposer, ShapeDecoder, SurroundingText,
    SwipeDecoder, SwipeLexicon, TraditionalVariant, Transcriber,
};

use crate::frame::{Frame, FrameCandidate};
use dictionaries::Mounted;

/// 软键盘的操作翻成 Engine 调用；每个操作返回操作之后的一帧。
pub struct Session {
    engine: Engine,

    /// 上一帧的候选，各个按下标选的操作从这里取。
    candidates: CandidateList,

    preedit: String,

    /// 光标在 `preedit` 里的字符下标。
    preedit_cursor: usize,

    /// `preedit` 里九宫格选定的音节（字符下标，起止），见 [`Frame::preedit_pinned`]。
    preedit_pinned: (usize, usize),

    pinyin_choices: Vec<String>,

    /// 不在组句时候选条上的补全（邮箱后缀、算式结果）：上屏前删掉光标前几个字符、要上屏的文字，下标同帧里的候选；空即没在补全。
    completions: Vec<(usize, String)>,

    /// 日语罗马音模式的缓冲区；`Some` 时敲键、选词都走它，不经拼音引擎（还没有日语词库，见 Core 的 `japanese`）。
    kana: Option<RomajiComposer>,

    /// 五笔码表的位置（随包数据里的 `wubi86.qj`）；切到五笔时才读。
    code_table: PathBuf,

    /// 随包数据目录，换学习语言时从这里装释义表。
    data_dir: PathBuf,

    /// 学习数据目录（个人释义表在这里）。
    user_dir: PathBuf,

    /// 功能面板「繁体」开着时用的字形（键盘设置里选）。
    traditional_variant: TraditionalVariant,

    /// 挂着的附加词库，见 [`Self::sync_dictionaries`]。
    dictionaries: Vec<Mounted>,

    /// 这段组句的光标前后文，发云联想时用；见 [`Self::set_surrounding`]。
    surrounding: Option<SurroundingText>,

    /// 插进候选的整句补全在第几个；选它走 `accept_prediction`。
    sentence_at: Option<usize>,

    /// 上屏通讯录里的名字后给不给这个人的电话、邮箱（键盘设置里的开关）。
    contact_details: bool,

    /// 刚上屏的通讯录名字，候选条上正放着它的电话、邮箱；光标前的文字不再以它结尾、或有别的操作就清掉。
    after_contact: Option<String>,

    /// 中文词上屏后候选条给不给联想词（键盘设置里的开关）。
    follow_ups: bool,

    /// 候选条上放着联想词时，刚上屏的那段文字；光标前的文字不再以它结尾、或有别的操作就清掉。
    after_commit: Option<String>,

    /// 联想词在 `completions` 里从第几个开始；选到这之后的走 [`qingjian_core::Engine::commit_follow_up`]。
    follow_up_at: Option<usize>,

    /// 删除键上划刚清掉的文字，候选条上放着「撤销」；光标前又有字、或有别的操作就清掉。
    cleared: Option<String>,

    /// 快捷翻译、常用语编辑这些工具框开着：打的字不进应用，不发云联想。
    tools_open: bool,

    /// Rust 侧的语音识别方（云端、离线模型）；系统识别器在壳里，不经这里。
    transcriber: Box<dyn Transcriber>,

    /// 滑行轨迹的解码。
    swipe_decoder: Box<dyn SwipeDecoder>,

    /// 滑行解拼音时比的音节串（主词库里各词的拼音），第一次滑行时才从词库建。
    swipe_pinyin: OnceLock<SwipeLexicon>,

    /// 滑行解英文时比的词（英文词表），同样用到才建。
    swipe_english: OnceLock<SwipeLexicon>,
}

impl Session {
    pub fn new(engine: Engine, code_table: PathBuf, data_dir: PathBuf, user_dir: PathBuf) -> Self {
        Self {
            code_table,
            data_dir,
            user_dir,
            engine,
            candidates: CandidateList::default(),
            preedit: String::new(),
            preedit_cursor: 0,
            preedit_pinned: (0, 0),
            pinyin_choices: Vec::new(),
            completions: Vec::new(),
            kana: None,
            traditional_variant: TraditionalVariant::Taiwan,
            dictionaries: Vec::new(),
            surrounding: None,
            sentence_at: None,
            tools_open: false,
            contact_details: false,
            after_contact: None,
            follow_ups: true,
            after_commit: None,
            follow_up_at: None,
            cleared: None,
            transcriber: Box::new(NoTranscriber),
            swipe_decoder: Box::new(ShapeDecoder),
            swipe_pinyin: OnceLock::new(),
            swipe_english: OnceLock::new(),
        }
    }

    /// 输入框换了或键盘收起：丢掉组句、断开上文、学习数据落盘。
    pub fn reset(&mut self) {
        self.engine.clear();
        self.engine.break_chain();
        self.engine.note_displayed(std::iter::empty());
        self.engine.flush_learning();
        self.candidates.items.clear();
        self.preedit.clear();
        self.preedit_cursor = 0;
        self.pinyin_choices.clear();
        self.surrounding = None;
        self.sentence_at = None;
        self.completions.clear();
        self.after_contact = None;
        self.after_commit = None;
        self.follow_up_at = None;
        self.cleared = None;
        if let Some(kana) = &mut self.kana {
            kana.clear();
        }
    }

    fn refresh(&mut self, commit: String) -> Frame {
        self.pinyin_choices.clear();
        self.preedit_pinned = (0, 0);
        self.completions.clear();
        self.after_contact = None;
        self.after_commit = None;
        self.follow_up_at = None;
        self.cleared = None;
        if let Some(kana) = &self.kana {
            self.preedit = kana.preedit();
            self.preedit_cursor = self.preedit.chars().count();
            let candidates = kana
                .candidates()
                .iter()
                .map(|text| FrameCandidate::plain(text))
                .collect();
            self.candidates.items.clear();
            return Frame {
                commit,
                replace: 0,
                preedit: self.preedit.clone(),
                preedit_cursor: self.preedit_cursor,
                preedit_pinned: (0, 0),
                notice: String::new(),
                pinyin_choices: Vec::new(),
                candidates,
                expression: false,
                single_chars: false,
            };
        }
        if self.engine.composition().is_empty() {
            self.candidates.items.clear();
            self.preedit.clear();
            self.preedit_cursor = 0;
        } else {
            match self.engine.query() {
                Ok(mut query) => {
                    query.candidates.items.truncate(Frame::MAX_CANDIDATES);
                    self.engine.annotate(&mut query.candidates);
                    self.preedit = query.marked_text();
                    self.preedit_cursor = query.marked_cursor();
                    let held = query.held.chars().count();
                    self.preedit_pinned = (held, held + self.engine.nine_key_pinned());
                    self.pinyin_choices = std::mem::take(&mut query.pinyin_choices);
                    self.candidates = query.candidates;
                }
                // 切不动也要把敲的字母留在 preedit 里，否则壳看到空帧会卡住
                Err(error) => {
                    tracing::debug!(%error, "查询失败，退回原始输入");
                    self.preedit = self.engine.held_text() + self.engine.composition().text();
                    self.preedit_cursor = self.preedit.chars().count();
                    self.candidates.items.clear();
                }
            }
        }
        self.request_prediction();
        self.frame(commit)
    }

    /// 按现在的拼音与候选出一帧（不重查）。
    fn frame(&self, commit: String) -> Frame {
        Frame {
            commit,
            replace: 0,
            preedit: self.preedit.clone(),
            preedit_cursor: self.preedit_cursor,
            preedit_pinned: self.preedit_pinned,
            notice: String::new(),
            pinyin_choices: self.pinyin_choices.clone(),
            candidates: self
                .candidates
                .items
                .iter()
                .map(|candidate| {
                    FrameCandidate::from(candidate)
                        .with_contact(self.engine.is_contact(&candidate.text))
                })
                .collect(),
            expression: self.engine.expression_mode() || self.engine.unicode_entry(),
            single_chars: self.engine.composition().single_chars(),
        }
    }
}
