//! 接上 bamboo-slip：建界面、处理消息、后台任务完成时叫醒。

use std::sync::Arc;
use std::time::Instant;

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{App, FamilyList, FamilyTable, Key, RenderError, TextPainter, Ui, Waker};

use super::{SIZES, Settings};
use crate::library::{self, Library};
use crate::message::{FieldId, Message, Popup, SizeKind};
use crate::page_id::PageId;
use crate::stats::Stats;
use crate::system::action::{self, Action};
use crate::system::fonts;
use crate::view;

impl App for Settings {
    type Message = Message;

    fn view(
        &self,
        ui: &mut Ui<Message>,
        scale: f32,
        size: (u32, u32),
    ) -> Result<NodeId, RenderError> {
        view::view(ui, self, scale, size)
    }

    fn update(&mut self, message: Message) -> bool {
        let armed = self.armed.take();
        if !matches!(
            message,
            Message::Run(_)
                | Message::TestCloud
                | Message::CheckUpdate
                | Message::Import(_)
                | Message::VocabularyExport(_)
        ) {
            self.notice = None;
        }
        match message {
            Message::Page(page) => {
                // 输入框在别的页上会直接消失、收不到失焦，这里补存
                for id in FieldId::ALL {
                    self.commit(id);
                }
                self.page = page;
                self.recording = None;
                self.close_phrase();
                if page == PageId::Dictionaries {
                    self.library = Library::scan();
                }
                if page == PageId::Stats {
                    self.stats = Some(Stats::load(&self.config.general));
                }
                if page == PageId::Vocabulary {
                    self.open_vocabulary();
                }
            }
            Message::SystemCtrlSpace(on) => self.set_ctrl_space(on),
            Message::Record(shortcut) => self.recording = Some(shortcut),
            Message::StopRecording => self.recording = None,
            Message::Recorded(shortcut, press) => match shortcut.accept(&press, &self.config) {
                Ok(value) => {
                    let (section, key) = shortcut.config_key();
                    self.write(section, key, value);
                    self.recording = None;
                }
                Err(reason) => self.notice = Some((shortcut.title(), reason)),
            },
            Message::ResetShortcuts => self.reset_shortcuts(),
            Message::OpenPhrase(index) => self.open_phrase(index),
            Message::PhrasePosition(delta) => {
                if let Some(editor) = self.phrase.as_mut() {
                    editor.position = editor
                        .position
                        .saturating_add_signed(delta as isize)
                        .clamp(1, 9);
                }
            }
            Message::SavePhrase => self.save_phrase(false),
            Message::DeletePhrase => {
                let confirmed = self
                    .phrase
                    .as_ref()
                    .is_some_and(|editor| editor.confirm_delete);
                if confirmed {
                    self.save_phrase(true);
                } else if let Some(editor) = self.phrase.as_mut() {
                    editor.confirm_delete = true;
                }
            }
            Message::ClosePhrase => self.close_phrase(),
            Message::TogglePhrase(index) => {
                let mut list = self.config.custom_phrases.clone();
                if let Some(phrase) = list.get_mut(index) {
                    phrase.enabled = !phrase.enabled;
                    self.write_phrases(&list).unwrap_or_else(|error| {
                        self.notice = Some(("phrases", error));
                    });
                }
            }
            Message::Import(kind) => match self.import.start(kind, self.waker.clone()) {
                Ok(true) => self.notice = Some((kind.slot(), "正在导入…".to_owned())),
                Ok(false) => {}
                Err(error) => self.notice = Some((kind.slot(), error)),
            },
            Message::Remove(kind, stem) => {
                if let Err(error) = library::remove(kind, &stem) {
                    self.notice = Some((kind.slot(), error));
                }
                self.library = Library::scan();
            }
            Message::ModeKey(key, letter) => {
                let mode = self.config.shortcut.mode.sanitized();
                let (current, other, other_key) = if key == "expression" {
                    (mode.expression, mode.question, "question")
                } else {
                    (mode.question, mode.expression, "expression")
                };
                if letter == other {
                    self.write("shortcut", other_key, current.to_string());
                }
                self.write("shortcut", key, letter.to_string());
            }
            Message::VocabularyView(_)
            | Message::VocabularyLanguage(_)
            | Message::VocabularyDay(_)
            | Message::VocabularyFilter(_)
            | Message::VocabularySort(_)
            | Message::VocabularyPage(_)
            | Message::VocabularyKnown(..)
            | Message::VocabularyRemove(_)
            | Message::VocabularyClear
            | Message::VocabularyExport(_) => self.vocabulary_message(message),
            Message::Edit(id, edit) => {
                if id == FieldId::VocabularySearch
                    && let Some(vocabulary) = self.vocabulary.as_mut()
                {
                    vocabulary.page = 0;
                }
                if let Some(copied) = self
                    .fields
                    .get_mut(&id)
                    .and_then(|field| field.apply(&edit))
                {
                    bamboo_slip::set_clipboard(&copied);
                }
            }
            Message::CheckUpdate => {
                if !self.update.running() {
                    let started = self.update.start(&self.config.update, self.waker.clone());
                    self.notice = Some((
                        "update",
                        started.err().unwrap_or_else(|| "正在检查…".to_owned()),
                    ));
                }
            }
            Message::Commit(id) => self.commit(id),
            Message::Cancel(id) => self.reset_field(id),
            Message::TestCloud => {
                if !self.cloud_test.running() {
                    let text = match self
                        .cloud_test
                        .start(&self.config.predict, self.waker.clone())
                    {
                        Ok(()) => "正在测试…".to_owned(),
                        Err(error) => error,
                    };
                    self.notice = Some(("cloud", text));
                }
            }
            Message::Set(section, key, value) => {
                self.menu = None;
                self.write(section, key, value);
            }
            Message::SetList(section, key, values) => self.write_list(section, key, &values),
            Message::OpenMenu(popup) => {
                if popup == Popup::Font && self.fonts.is_none() {
                    self.fonts = Some(fonts::families());
                }
                self.menu = Some(popup);
            }
            Message::CloseMenu => self.menu = None,
            Message::Size(kind, delta) => {
                let value = (self.size(kind) + delta as f32).clamp(*SIZES.start(), *SIZES.end());
                let key = match kind {
                    SizeKind::Candidate => "candidate_font_size",
                    SizeKind::Annotation => "annotation_font_size",
                };
                self.write("general", key, f64::from(value));
            }
            Message::ResetSizes => {
                self.write("general", "candidate_font_size", 0.0);
                self.write("general", "annotation_font_size", 0.0);
            }
            Message::Run(action) => {
                // 清空要点两下：第一下只是把按钮换成确认
                if action == Action::ClearInputLog && armed != Some(action) {
                    self.armed = Some(action);
                    self.notice = None;
                } else {
                    self.notice = action.run(&self.path).map(|text| (action.slot(), text));
                }
            }
            Message::ToggleSources => self.sources_open = !self.sources_open,
            Message::StatsCloud(cloud) => self.stats_cloud = cloud,
            Message::Search(edit) => {
                self.search.edit.apply(&edit);
                if !self.search.active() {
                    self.search.index = None;
                } else if self.search.index.is_none() {
                    self.search.index = Some(view::index(self));
                }
                self.search.refresh();
            }
            Message::SearchFirst => {
                if !self.search.hits.is_empty() {
                    return self.update(Message::Jump(0));
                }
            }
            Message::ClearSearch => self.search.clear(),
            Message::OpenDoc(url) => {
                if let Err(error) = action::open(&url) {
                    tracing::warn!(%error, url, "打开文档失败");
                }
            }
            Message::Jump(index) => {
                if let Some(hit) = self.search.hits.get(index).cloned() {
                    self.update(Message::Page(hit.page));
                    self.spotlight(hit.key);
                }
            }
            Message::Quit => return false,
        }
        true
    }

    fn setup(&mut self, text: &mut TextPainter) {
        let mut table = FamilyTable::new(FamilyList::system());
        self.serif = table.intern(&FamilyList(vec![
            "Songti SC".to_owned(),
            "SimSun".to_owned(),
            "Noto Serif CJK SC".to_owned(),
        ]));
        text.use_families(&Arc::new(table));
    }

    fn set_waker(&mut self, waker: Waker) {
        self.waker = Some(waker);
    }

    fn wake(&mut self) -> bool {
        let mut changed = self.search.expire(Instant::now());
        if let Some(text) = self.cloud_test.finish() {
            self.notice = Some(("cloud", text));
            changed = true;
        }
        if let Some(report) = self.import.finish() {
            self.imported(report);
            changed = true;
        }
        if let Some(text) = self.update.finish(&self.config.update) {
            self.notice = Some(("update", text));
            changed = true;
        }
        changed
    }

    fn set_system_dark(&mut self, dark: bool) {
        self.system_dark = dark;
    }

    fn spotlight(&self) -> Option<Key> {
        self.search.spotlight.map(|(key, _)| key)
    }
}
