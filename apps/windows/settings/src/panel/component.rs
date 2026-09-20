//! 根组件的 Reactor 生命周期：建状态、按消息落盘、画左侧导航 + 当前页。

use qingjian_platform::{
    CandidateRenderer, Config, DEFAULT_ENGLISH_CANDIDATES_OFF_WINDOWS, LayoutMode, LogLevel,
    PreeditMode, ShiftLetter, ThemeMode,
};
use windows_reactor::*;

use super::cloud_status::CloudStatus;
use super::controls::{export_logs, log_dir, open_in_editor, open_with_explorer};
use super::notice::Notice;
use super::pages::{about, aux_code, cloud, dictionaries, general, shortcut};
use super::recorder::Recorder;
use super::{Message, Settings};

impl Component for Settings {
    type Input = ();
    type Message = Message;

    fn create(_input: &(), _context: &ComponentContext<Self>) -> Self {
        let path = Self::config_path();
        Self::ensure_config_file(&path);
        let config = Config::load(&path).unwrap_or_default();
        Self {
            config,
            path,
            page: "general".to_string(),
            cloud_status: CloudStatus::Idle,
            recorder: Recorder::Idle,
            record_box: ElementRef::new(),
            notice: Notice::default(),
            dictionary_status: String::new(),
            families: qingjian_render::system_fonts::families(),
            font_query: None,
        }
    }

    fn update(&mut self, message: Message, context: &ComponentContext<Self>) {
        match message {
            Message::Navigate(Some(tag)) => {
                self.page = tag;
                // 上一页的导入提示不跟着过来
                self.notice.clear();
            }
            Message::Navigate(None) => {}

            // 通用页
            Message::LearningLanguage(Some(i)) if i < general::LANGUAGES.len() => {
                self.save("general", "learning_language", general::LANGUAGES[i].1);
            }
            Message::PageSize(Some(value)) => {
                let size = (value.round() as i64).clamp(1, 9);
                self.save("general", "page_size", size);
            }
            Message::Scheme(Some(i)) if i < general::SCHEMES.len() => {
                self.save("general", "scheme", general::SCHEMES[i].1);
            }
            Message::Wubi(on) => self.save("general", "wubi", if on { "wubi86" } else { "" }),
            Message::Traditional(on) => self.save("general", "traditional", on),
            Message::EnglishCandidates(on) => self.save("general", "english_candidates", on),
            Message::ChineseFirst(on) => self.save("general", "chinese_first", on),
            Message::FullWidthPunctuation(on) => {
                self.save("general", "full_width_punctuation", on);
            }
            Message::EnglishFullWidthPunctuation(on) => {
                self.save("general", "english_full_width_punctuation", on);
            }
            Message::EnglishOffInApps(on) => {
                let list: Vec<String> = if on {
                    DEFAULT_ENGLISH_CANDIDATES_OFF_WINDOWS
                        .iter()
                        .map(|s| (*s).to_owned())
                        .collect()
                } else {
                    Vec::new()
                };
                self.save_array("apps", "english_candidates_off", &list);
            }
            Message::SwitchMode(Some(i)) if i < general::SWITCH_KEYS.len() => {
                self.save("shortcut", "switch_mode", general::SWITCH_KEYS[i].1);
            }
            Message::EnglishMode(on) => self.save("general", "english_mode", on),

            // 候选窗口页
            Message::Theme(Some(i)) if i < ThemeMode::ALL.len() => {
                self.save("general", "theme", ThemeMode::ALL[i].key());
            }
            Message::Layout(Some(i)) if i < LayoutMode::ALL.len() => {
                self.save("general", "layout", LayoutMode::ALL[i].key());
            }
            Message::Preedit(Some(i)) if i < PreeditMode::ALL.len() => {
                self.save("general", "preedit", PreeditMode::ALL[i].key());
            }
            Message::ShiftLetter(Some(i)) if i < ShiftLetter::ALL.len() => {
                self.save("general", "shift_letter", ShiftLetter::ALL[i].key());
            }
            Message::Renderer(Some(i)) if i < CandidateRenderer::ALL.len() => {
                self.save("general", "renderer", CandidateRenderer::ALL[i].key());
            }
            Message::FontQuery(text) => {
                let text = text.trim().to_owned();
                let exact = self
                    .families
                    .iter()
                    .find(|family| family.eq_ignore_ascii_case(&text))
                    .cloned();
                match exact {
                    Some(family) => {
                        self.font_query = None;
                        self.save("general", "font", family);
                    }
                    None if text.is_empty() => {
                        self.font_query = None;
                        self.save("general", "font", "");
                    }
                    None => self.font_query = Some(text),
                }
            }
            Message::Font(family) => {
                self.font_query = None;
                self.save("general", "font", family);
            }
            Message::StatusBar(on) => self.save("status_bar", "enabled", on),

            // 云服务页
            Message::LocalModel(on) => self.save("model", "enabled", on),
            Message::CloudEnabled(on) => self.save("predict", "enabled", on),
            Message::CloudApiKey(value) => self.save("predict", "api_key", value),
            Message::CloudModel(value) => self.save("predict", "model", value),
            Message::CloudBaseUrl(value) => self.save("predict", "base_url", value),
            Message::CloudSlots(Some(value)) => {
                let slots = (value.round() as i64).clamp(0, 9);
                self.save("predict", "slots", slots);
            }
            Message::CloudSentence(on) => self.save("predict", "sentence", on),
            Message::TestConnection => {
                if matches!(self.cloud_status, CloudStatus::Testing) {
                    return;
                }
                self.cloud_status = CloudStatus::Testing;
                let config = self.config.predict.clone();
                context.spawn_background(move |cancel| {
                    Message::CloudTestDone(cloud::run_test(&config, &cancel))
                });
            }
            Message::CloudTestDone(result) => {
                self.cloud_status = match result {
                    Ok(message) => CloudStatus::Ok(message),
                    Err(message) => CloudStatus::Failed(message),
                };
            }

            // 快捷键页
            Message::PageKeys(Some(i)) if i < shortcut::PAGE_KEYS.len() => {
                self.save("general", "page_keys", shortcut::PAGE_KEYS[i].1);
            }
            Message::ModeExpression(Some(i)) if i < shortcut::MODE_KEYS.len() => {
                self.save("shortcut", "expression", shortcut::MODE_KEYS[i]);
            }
            Message::ModeQuestion(Some(i)) if i < shortcut::MODE_KEYS.len() => {
                self.save("shortcut", "question", shortcut::MODE_KEYS[i]);
            }
            Message::QuestionMark(on) => self.save("shortcut", "question_mark", on),
            Message::Translation(Some(i)) if i < shortcut::MODIFIERS.len() => {
                self.save("shortcut", "translation", shortcut::MODIFIERS[i].1);
            }
            Message::TranslationSecond(Some(i)) if i < shortcut::MODIFIERS.len() => {
                self.save("shortcut", "translation_second", shortcut::MODIFIERS[i].1);
            }
            Message::DeleteCandidate(Some(i)) if i < shortcut::MODIFIERS.len() => {
                self.save("shortcut", "delete_candidate", shortcut::MODIFIERS[i].1);
            }
            Message::TranslateSelection(Some(i)) if i < shortcut::MODIFIERS.len() => {
                let key = self.config.shortcut.translate_selection.key;
                let combo = format!("{}+{key}", shortcut::MODIFIERS[i].1);
                self.save("shortcut", "translate_selection", combo);
            }

            // 模糊音页
            Message::Fuzzy(key, on) => self.save("fuzzy", key, on),

            // 词库页
            Message::ToggleDomain(name, on) => {
                let mut domains = self.config.dictionaries.domains.clone();
                if on {
                    if !domains.contains(&name) {
                        domains.push(name);
                    }
                } else {
                    domains.retain(|d| d != &name);
                }
                self.save_array("dictionaries", "domains", &domains);
            }
            Message::ToggleUserDict(name, on) => {
                // 用户词库缺省启用，`disabled` 列的是关掉的。
                let mut disabled = self.config.dictionaries.disabled.clone();
                if on {
                    disabled.retain(|d| d != &name);
                } else if !disabled.contains(&name) {
                    disabled.push(name);
                }
                self.save_array("dictionaries", "disabled", &disabled);
            }
            Message::RemoveUserDict(name) => {
                dictionaries::remove_user_dict(self, &name);
                self.reload();
            }
            Message::ImportDictionary => {
                dictionaries::import(self);
                self.reload();
            }

            // 辅码页
            Message::AuxCodeEnabled(on) => self.save("aux_code", "enabled", on),
            Message::AuxCodeShow(on) => self.save("general", "aux_code_show", on),
            Message::AuxCodeKeepEmpty(on) => self.save("general", "aux_code_keep_empty", on),
            Message::AuxRecordStart => self.recorder = self.recorder.waiting(),
            Message::AuxRecordCancel => self.recorder = Recorder::Idle,
            Message::AuxRecorded(text) => aux_code::record_key(self, &text),
            Message::ToggleAuxTable(name, on) => {
                // 码表缺省启用，`disabled` 列的是关掉的；随包笔画表也走这条
                let mut disabled = self.config.aux_code.disabled.clone();
                if on {
                    disabled.retain(|d| d != &name);
                } else if !disabled.contains(&name) {
                    disabled.push(name);
                }
                self.save_array("aux_code", "disabled", &disabled);
            }
            Message::RemoveAuxTable(name) => {
                aux_code::remove_table(self, &name);
                self.reload();
            }
            Message::ImportCodeTable => {
                aux_code::import(self);
                self.reload();
            }

            // 高级页
            Message::VerboseLog(on) => {
                let level = if on { LogLevel::Debug } else { LogLevel::Info };
                self.save("general", "log_level", level.key());
            }
            Message::InputLog(on) => self.save("general", "input_log", on),
            Message::Learning(on) => self.save("general", "learning", on),
            Message::OpenConfigFile => {
                Self::ensure_config_file(&self.path);
                open_in_editor(&self.path);
            }
            Message::OpenDataDir => {
                Self::ensure_config_file(&self.path);
                open_with_explorer(&self.data_dir().to_string_lossy());
            }
            Message::OpenLogDir => {
                if let Some(logs) = log_dir() {
                    open_with_explorer(&logs.to_string_lossy());
                }
            }
            Message::ExportLogs => export_logs(),
            Message::ClearInputLog => {
                let log = self.data_dir().join("input-log.jsonl");
                if let Err(error) = std::fs::remove_file(&log)
                    && error.kind() != std::io::ErrorKind::NotFound
                {
                    crate::log::warn(format!("清空输入日志失败: {error}"));
                }
            }

            // 关于页
            Message::OpenWebsite => open_with_explorer(about::WEBSITE_URL),
            Message::OpenRepository => open_with_explorer(about::REPOSITORY_URL),

            // 下拉被清空 / 越界：不改
            _ => {}
        }
    }

    fn view(&self, _input: &(), context: &mut ViewContext<Self>) -> View {
        context.window_title("青简设置");
        let item = |tag: &str, label: &str, symbol| {
            KeyedView::new(
                tag,
                NavigationViewItem::new()
                    .tag(tag)
                    .is_selected(self.page == tag)
                    .slots([
                        SlotView::new(
                            NavigationViewItemSlot::Icon,
                            SymbolIcon::new().symbol(symbol),
                        ),
                        SlotView::new(NavigationViewItemSlot::Content, label),
                    ]),
            )
        };
        let items = [
            item("general", "通用", Symbol::Setting),
            item("candidates", "候选窗口", Symbol::View),
            item("shortcut", "快捷键", Symbol::Keyboard),
            item("cloud", "云服务", Symbol::World),
            item("fuzzy", "模糊音", Symbol::Audio),
            item("dictionaries", "词库", Symbol::Library),
            item("aux_code", "辅码", Symbol::Character),
            item("usage", "统计", Symbol::List),
            item("advanced", "高级", Symbol::Repair),
            item("about", "关于", Symbol::Help),
        ];
        NavigationView::new()
            .pane_display_mode(NavigationViewPaneDisplayMode::Left)
            .pane_title("青简")
            .open_pane_length(220.0)
            .is_pane_open(true)
            .is_pane_toggle_button_visible(false)
            .is_back_button_visible(NavigationViewBackButtonVisible::Collapsed)
            .is_settings_visible(false)
            .on_selected_tag_changed(context.callback(Message::Navigate))
            .slots([
                SlotView::collection(NavigationViewSlot::MenuItems, items),
                SlotView::new(NavigationViewSlot::Content, self.page_content(context)),
            ])
    }
}
