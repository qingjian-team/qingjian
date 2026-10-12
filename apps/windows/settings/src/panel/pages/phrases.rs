//! 「常用语」页：管理 `[[custom_phrases]]`——输入码敲全后固定位置出候选。
//! 新增时输入码缺省取内容前三个字的拼音首字母（或英文字母）；用户改过码就不再自动跟。

use std::collections::HashMap;
use std::sync::OnceLock;

use qingjian_core::custom_phrase::{self, CustomPhrase};
use qingjian_dictionary::Dictionary;
use windows_reactor::*;

use crate::panel::controls::{feedback, field, note, page, repo_resource};
use crate::panel::{Message, Settings};

/// 编辑中的草稿；`None` 表示列表态。
#[derive(Debug, Clone)]
pub(crate) struct PhraseEditor {
    /// 正在改的下标；`None` 为新增。
    pub(crate) index: Option<usize>,

    pub(crate) text: String,

    pub(crate) code: String,

    /// 用户手改过输入码后为真，内容变化不再刷码。
    pub(crate) code_touched: bool,

    /// 1–9。
    pub(crate) position: usize,

    pub(crate) enabled: bool,

    pub(crate) error: String,
}

impl PhraseEditor {
    pub(crate) fn new_blank() -> Self {
        Self {
            index: None,
            text: String::new(),
            code: String::new(),
            code_touched: false,
            // 缺省第 2 位：第 1 位留给拼音首选词，常用语跟在后面（与微信输入法习惯接近）
            position: 2,
            enabled: true,
            error: String::new(),
        }
    }

    pub(crate) fn from_phrase(index: usize, phrase: &CustomPhrase) -> Self {
        Self {
            index: Some(index),
            text: phrase.text.clone(),
            code: phrase.code.clone(),
            code_touched: true,
            position: phrase.position,
            enabled: phrase.enabled,
            error: String::new(),
        }
    }

    fn into_phrase(self) -> CustomPhrase {
        CustomPhrase {
            code: custom_phrase::normalize_code(self.code.trim()),
            text: self.text,
            position: self.position,
            enabled: self.enabled,
        }
    }
}

/// 单字 → 拼音首字母；惰性从基础词库建，建不出就空表（只剩英文字母建议）。
fn char_initial(ch: char) -> Option<char> {
    static MAP: OnceLock<HashMap<char, char>> = OnceLock::new();
    let map = MAP.get_or_init(|| {
        let mut map = HashMap::new();
        let Some(path) = dict_qj_path() else {
            return map;
        };
        let Ok(dict) = Dictionary::from_path(&path) else {
            return map;
        };
        for entry in dict.entries() {
            let mut chars = entry.text.chars();
            let Some(han) = chars.next() else {
                continue;
            };
            if chars.next().is_some() {
                continue;
            }
            let Some(first) = entry.pinyin.split_whitespace().next() else {
                continue;
            };
            let Some(initial) = first.chars().next().filter(|c| c.is_ascii_alphabetic()) else {
                continue;
            };
            map.entry(han).or_insert(initial.to_ascii_lowercase());
        }
        map
    });
    map.get(&ch).copied()
}

/// 随包根 → 联调目录 → 安装目录，谁有 `dict.qj` 用谁。
fn dict_qj_path() -> Option<std::path::PathBuf> {
    if let Some(path) = repo_resource("data/generated/dict.qj") {
        return Some(path);
    }
    let candidates = [
        std::env::var_os("USERPROFILE")
            .map(std::path::PathBuf::from)
            .map(|home| home.join(r"qingjian-devtest\data\generated\dict.qj")),
        Some(std::path::PathBuf::from(
            r"C:\Program Files\Qingjian\data\generated\dict.qj",
        )),
    ];
    candidates
        .into_iter()
        .flatten()
        .find(|path| path.is_file())
}

fn suggest_for(text: &str) -> String {
    custom_phrase::suggest_code(text, char_initial)
}

fn list_rows(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let phrases = &settings.config.custom_phrases;
    if phrases.is_empty() {
        return note("还没有常用语。点下面「添加」写一条；敲全输入码后会出现在固定候选位置。");
    }
    let mut rows: Vec<KeyedView> = Vec::with_capacity(phrases.len());
    for (index, phrase) in phrases.iter().enumerate() {
        let preview = CustomPhrase::preview(&phrase.text, 40);
        let label = format!(
            "{preview}    ·  码 {code}  ·  第 {pos} 位",
            code = phrase.code,
            pos = phrase.position
        );
        let toggle = {
            let i = index;
            move |on| Message::PhraseToggle(i, on)
        };
        let check = CheckBox::new()
            .is_checked(phrase.enabled)
            .on_is_checked_changed(context.callback(toggle))
            .content(label);
        let row = StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .children((
                check,
                Button::new()
                    .on_click(context.message(Message::PhraseEdit(index)))
                    .content("编辑"),
                Button::new()
                    .on_click(context.message(Message::PhraseDelete(index)))
                    .content("删除"),
            ));
        rows.push(KeyedView::new(format!("phrase-{index}-{}", phrase.code), row));
    }
    StackPanel::new().spacing(6.0).keyed_children(rows)
}

fn editor_view(editor: &PhraseEditor, context: &mut ViewContext<Settings>) -> View {
    let title = if editor.index.is_some() {
        "编辑常用语"
    } else {
        "添加常用语"
    };
    let positions: Vec<String> = (1..=9).map(|n| format!("第 {n} 位")).collect();
    let mut body: Vec<KeyedView> = vec![
        KeyedView::new(
            "title",
            TextBlock::new()
                .text(title)
                .font_weight(FontWeight::SEMI_BOLD),
        ),
        KeyedView::new(
            "text",
            field(
                "内容",
                "",
                TextBox::new()
                    .text(editor.text.clone())
                    .placeholder_text("输入常用语")
                    .width(480.0)
                    .on_text_changed(context.callback(Message::PhraseDraftText)),
            ),
        ),
        KeyedView::new(
            "code",
            field(
                "输入码",
                "1–32 个小写字母、数字或汉字。留空或跟着内容走时，缺省取内容前三个字（字母收小写）。",
                TextBox::new()
                    .text(editor.code.clone())
                    .placeholder_text("默认：内容前 3 字")
                    .width(480.0)
                    .on_text_changed(context.callback(Message::PhraseDraftCode)),
            ),
        ),
        KeyedView::new(
            "position",
            field(
                "候选位置",
                "同一输入码下各条位置不能重复。",
                ComboBox::new()
                    .items_source(positions)
                    .selected_index(editor.position.saturating_sub(1))
                    .on_selection_changed(context.callback(Message::PhraseDraftPosition)),
            ),
        ),
        KeyedView::new(
            "enabled",
            field(
                "启用",
                "",
                ToggleSwitch::new()
                    .is_on(editor.enabled)
                    .on_toggled(context.callback(Message::PhraseDraftEnabled)),
            ),
        ),
    ];
    if !editor.error.is_empty() {
        body.push(KeyedView::new(
            "error",
            TextBlock::new()
                .text(editor.error.clone())
                .text_wrapping(TextWrapping::Wrap)
                .font_size(12.0)
                .foreground(ThemeBrush::SystemCritical),
        ));
    }
    body.push(KeyedView::new(
        "actions",
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(12.0)
            .horizontal_alignment(HorizontalAlignment::Right)
            .children((
                Button::new()
                    .on_click(context.message(Message::PhraseCancel))
                    .content("取消"),
                Button::new()
                    .on_click(context.message(Message::PhraseSave))
                    .content(if editor.index.is_some() {
                        "完成"
                    } else {
                        "添加"
                    }),
            )),
    ));
    StackPanel::new().spacing(12.0).keyed_children(body)
}

pub(crate) fn view(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let body = if let Some(editor) = &settings.phrase_editor {
        editor_view(editor, context)
    } else {
        StackPanel::new()
            .spacing(12.0)
            .children([
                note(
                    "字母 / 数字码：敲全输入码后出现。汉字码：打出该词的拼音、候选里出现这个词时也会插入。数字键或空格上屏；中文与英文模式都生效。改完自动生效。",
                ),
                list_rows(settings, context),
                StackPanel::new()
                    .orientation(Orientation::Horizontal)
                    .spacing(12.0)
                    .horizontal_alignment(HorizontalAlignment::Right)
                    .children((Button::new()
                        .on_click(context.message(Message::PhraseNew))
                        .content("添加"),))
                    .into(),
                feedback(&settings.notice),
            ])
    };
    page("常用语", body)
}

pub(crate) fn open_new(settings: &mut Settings) {
    settings.phrase_editor = Some(PhraseEditor::new_blank());
    settings.notice.clear();
}

pub(crate) fn open_edit(settings: &mut Settings, index: usize) {
    let Some(phrase) = settings.config.custom_phrases.get(index).cloned() else {
        return;
    };
    settings.phrase_editor = Some(PhraseEditor::from_phrase(index, &phrase));
    settings.notice.clear();
}

pub(crate) fn draft_text(settings: &mut Settings, text: String) {
    let Some(editor) = settings.phrase_editor.as_mut() else {
        return;
    };
    editor.text = text;
    if !editor.code_touched {
        editor.code = suggest_for(&editor.text);
    }
    editor.error.clear();
}

pub(crate) fn draft_code(settings: &mut Settings, code: String) {
    let Some(editor) = settings.phrase_editor.as_mut() else {
        return;
    };
    editor.code = code;
    editor.code_touched = true;
    editor.error.clear();
}

pub(crate) fn draft_position(settings: &mut Settings, index: Option<usize>) {
    let Some(editor) = settings.phrase_editor.as_mut() else {
        return;
    };
    if let Some(i) = index.filter(|i| *i < 9) {
        editor.position = i + 1;
        editor.error.clear();
    }
}

pub(crate) fn draft_enabled(settings: &mut Settings, on: bool) {
    if let Some(editor) = settings.phrase_editor.as_mut() {
        editor.enabled = on;
    }
}

pub(crate) fn cancel(settings: &mut Settings) {
    settings.phrase_editor = None;
}

pub(crate) fn toggle(settings: &mut Settings, index: usize, on: bool) {
    let mut phrases = settings.config.custom_phrases.clone();
    let Some(phrase) = phrases.get_mut(index) else {
        return;
    };
    phrase.enabled = on;
    persist(settings, phrases, None);
}

pub(crate) fn delete(settings: &mut Settings, index: usize) {
    let mut phrases = settings.config.custom_phrases.clone();
    if index >= phrases.len() {
        return;
    }
    phrases.remove(index);
    persist(settings, phrases, Some("已删除。".into()));
}

pub(crate) fn save(settings: &mut Settings) {
    let Some(editor) = settings.phrase_editor.clone() else {
        return;
    };
    let index = editor.index;
    let phrase = editor.into_phrase();
    let mut phrases = settings.config.custom_phrases.clone();
    match index {
        Some(i) if i < phrases.len() => phrases[i] = phrase,
        Some(_) => {
            if let Some(editor) = settings.phrase_editor.as_mut() {
                editor.error = "这条常用语已不存在，请取消后重试。".into();
            }
            return;
        }
        None => phrases.push(phrase),
    }
    if let Err(error) = custom_phrase::validate_phrases(&phrases) {
        if let Some(editor) = settings.phrase_editor.as_mut() {
            editor.error = error;
        }
        return;
    }
    if persist(settings, phrases, Some("已保存。".into())) {
        settings.phrase_editor = None;
    }
}

/// 写回配置；失败时把原因写进编辑器或页面提示。成功返回 true。
fn persist(settings: &mut Settings, phrases: Vec<CustomPhrase>, ok_note: Option<String>) -> bool {
    match qingjian_platform::Config::set_custom_phrases(&settings.path, &phrases) {
        Ok(()) => {
            settings.reload();
            if let Some(note) = ok_note {
                settings.notice.succeed(note);
            }
            true
        }
        Err(error) => {
            if let Some(editor) = settings.phrase_editor.as_mut() {
                editor.error = error;
            } else {
                settings.notice.fail(error);
            }
            false
        }
    }
}
