//! 设置窗口的根组件：左侧导航栏（[`NavigationView`]）+ 右侧当前分节的内容页。
//!
//! 每改一项就 [`Config::set_value`]（列表用 [`Config::set_array`]）原地落盘（保留注释），再从盘上重读一份，
//! 保证界面与文件一致。Server 只在启动时读一次配置，改完要重启 Server 才生效（见 README）。
//! 分节对齐 macOS 偏好设置：通用 / 候选窗口 / 快捷键 / 云服务 / 模糊音 / 词库 / 高级 / 关于，
//! 各页在子模块里（[`general`] / [`candidates`] / [`shortcut`] / [`cloud`] / [`fuzzy`] / [`dictionaries`] / [`advanced`] / [`about`]）。

mod about;
mod advanced;
mod candidates;
mod cloud;
mod dictionaries;
mod fuzzy;
mod general;
mod shortcut;
mod usage;

use std::path::{Path, PathBuf};

use qingjian_platform::{
    Config, DEFAULT_ENGLISH_CANDIDATES_OFF_WINDOWS, LayoutMode, LogLevel, PreeditMode, ThemeMode,
};
use windows_reactor::*;

/// 左侧标签的固定宽度，让各行控件对齐。
const LABEL_WIDTH: f64 = 220.0;

/// 「测试连接」的状态。
#[derive(Clone)]
enum CloudStatus {
    /// 没测过。
    Idle,
    /// 正在测。
    Testing,
    /// 成功（带一行说明）。
    Ok(String),
    /// 失败（带错误说明）。
    Failed(String),
}

/// 设置窗口状态。
pub(crate) struct Settings {
    /// 当前配置（每次改动后从盘上重读，供各页显示当前值）。
    pub(super) config: Config,

    /// `config.toml` 路径（`%APPDATA%\Qingjian\config.toml`），落盘时用。
    path: PathBuf,

    /// 当前选中的导航分节 tag。
    page: String,

    /// 云服务「测试连接」的状态。
    cloud_status: CloudStatus,
}

/// 设置窗口的消息。每个「改动」消息带上控件的新值，[`Settings::update`] 据此落盘。
#[derive(Clone)]
pub(crate) enum Message {
    /// 左侧导航切换分节（`None` 是取消选中，忽略）。
    Navigate(Option<String>),

    // —— 通用页 ——
    LearningLanguage(Option<usize>),
    PageSize(Option<f64>),
    Shuangpin(Option<usize>),
    EnglishCandidates(bool),
    /// 在终端 / 编辑器里不给英文候选（开=写入平台默认名单，关=清空）。
    EnglishOffInApps(bool),

    // —— 候选窗口页 ——
    Theme(Option<usize>),
    Layout(Option<usize>),
    Preedit(Option<usize>),

    // —— 云服务页 ——
    CloudEnabled(bool),
    CloudApiKey(String),
    CloudModel(String),
    CloudBaseUrl(String),
    CloudSlots(Option<f64>),
    CloudSentence(bool),
    /// 点「测试连接」。
    TestConnection,
    /// 后台测试连接完成。
    CloudTestDone(Result<String, String>),

    // —— 快捷键页 ——
    PageKeys(Option<usize>),
    ModeExpression(Option<usize>),
    ModeQuestion(Option<usize>),
    Translation(Option<usize>),
    TranslationSecond(Option<usize>),
    DeleteCandidate(Option<usize>),

    // —— 模糊音页 ——
    /// 一条模糊音规则开关（配置键 + 新值）。
    Fuzzy(&'static str, bool),

    // —— 词库页 ——
    /// 开关一本随包领域词库（词库名 + 新值）。
    ToggleDomain(String, bool),
    /// 开关一本用户导入词库（词库名 + 新值）。
    ToggleUserDict(String, bool),
    /// 移除一本用户导入词库（挪进 dicts\removed）。
    RemoveUserDict(String),
    /// 导入词库文件（弹文件选择器，拷进用户词库目录）。
    ImportDictionary,

    // —— 高级页 ——
    /// 详细日志（debug 级）开关。
    VerboseLog(bool),
    /// 记录输入日志开关。
    InputLog(bool),
    /// 在编辑器里打开配置文件。
    OpenConfigFile,
    /// 打开数据目录（`%APPDATA%\Qingjian`）。
    OpenDataDir,
    /// 打开日志目录（`%APPDATA%\Qingjian\logs`）。
    OpenLogDir,
    /// 清空输入日志。
    ClearInputLog,

    // —— 关于页 ——
    OpenWebsite,
    OpenRepository,
}

impl Settings {
    /// `config.toml` 的位置：`%APPDATA%\Qingjian\config.toml`；取不到 `APPDATA` 退回工作目录。
    fn config_path() -> PathBuf {
        std::env::var_os("APPDATA")
            .map(|dir| PathBuf::from(dir).join("Qingjian").join("config.toml"))
            .unwrap_or_else(|| PathBuf::from("config.toml"))
    }

    /// 数据目录（`%APPDATA%\Qingjian`）：配置、密钥、输入日志、统计都在这里。
    fn data_dir(&self) -> &Path {
        self.path.parent().unwrap_or_else(|| Path::new("."))
    }

    /// 落盘一个配置值（保留注释），再从盘上重读，让界面与文件保持一致。失败只打印，不致命。
    fn save(&mut self, section: &str, key: &str, value: impl Into<toml_edit::Value>) {
        if let Err(error) = Config::set_value(&self.path, section, key, value) {
            eprintln!("保存 [{section}] {key} 失败: {error}");
            return;
        }
        self.reload();
    }

    /// 落盘一个字符串数组配置（词库列表 / 应用名单），再重读。
    fn save_array(&mut self, section: &str, key: &str, values: &[String]) {
        if let Err(error) = Config::set_array(&self.path, section, key, values) {
            eprintln!("保存 [{section}] {key} 失败: {error}");
            return;
        }
        self.reload();
    }

    /// 从盘上重读配置到内存。
    fn reload(&mut self) {
        if let Ok(config) = Config::load(&self.path) {
            self.config = config;
        }
    }

    /// 当前分节的内容页。
    fn page_content(&self, context: &mut ViewContext<Self>) -> View {
        match self.page.as_str() {
            "candidates" => candidates::view(self, context),
            "shortcut" => shortcut::view(self, context),
            "cloud" => cloud::view(self, context),
            "fuzzy" => fuzzy::view(self, context),
            "dictionaries" => dictionaries::view(self, context),
            "usage" => usage::view(self, context),
            "advanced" => advanced::view(self, context),
            "about" => about::view(self, context),
            _ => general::view(self, context),
        }
    }
}

/// 随包资源目录（dev 布局）：exe 在 `ime\target\debug\` 下，仓库根是往上三层，再接 `rel`。
/// 找不到（换成安装布局后）返回 `None`。等 Windows 定了固定安装路径再改这里。
pub(super) fn repo_resource(rel: &str) -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let root = exe.ancestors().nth(3)?; // exe → debug → target → ime
    let path = root.join(rel);
    path.exists().then_some(path)
}

/// 用记事本打开一个文件（配置文件）。失败只打印。
fn open_in_editor(path: &Path) {
    if let Err(error) = std::process::Command::new("notepad").arg(path).spawn() {
        eprintln!("打开 {} 失败: {error}", path.display());
    }
}

/// 用资源管理器打开一个目录或网址。失败只打印。
fn open_with_explorer(target: &str) {
    if let Err(error) = std::process::Command::new("explorer").arg(target).spawn() {
        eprintln!("打开 {target} 失败: {error}");
    }
}

/// 一行设置：左侧固定宽标签 + 右侧控件。各页共用。
pub(super) fn labeled(label: &str, control: impl Into<View>) -> View {
    StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(12.0)
        .children([
            TextBlock::new().text(label).width(LABEL_WIDTH).into(),
            control.into(),
        ])
}

/// 一条灰色小字说明（对齐 macOS 各项下的注释）。可换行。
pub(super) fn note(text: &str) -> View {
    TextBlock::new()
        .text(text)
        .text_wrapping(TextWrapping::Wrap)
        .font_size(12.0)
        .opacity(0.6)
        .into()
}

/// 一整项设置：一行「标签 + 控件」，下面接一条灰色说明（`hint` 为空则不加说明）。各页共用。
pub(super) fn field(label: &str, hint: &str, control: impl Into<View>) -> View {
    let row = labeled(label, control);
    if hint.is_empty() {
        row
    } else {
        StackPanel::new().spacing(4.0).children([row, note(hint)])
    }
}

/// 在 `(界面名, 配置写法)` 列表里找 `value` 的下标，找不到取 0。各页共用。
pub(super) fn index_of(options: &[(&str, &str)], value: &str) -> usize {
    options.iter().position(|(_, v)| *v == value).unwrap_or(0)
}

/// 一页的外壳：可滚动 + 大标题 + 内容，统一边距与间距。
pub(super) fn page(title: &str, body: impl Into<View>) -> View {
    ScrollViewer::new().content(
        StackPanel::new().spacing(16.0).margin(24.0).children([
            TextBlock::new()
                .text(title)
                .font_size(24.0)
                .font_weight(FontWeight::SEMI_BOLD)
                .into(),
            body.into(),
        ]),
    )
}

impl Component for Settings {
    type Input = ();
    type Message = Message;

    fn create(_input: &(), _context: &ComponentContext<Self>) -> Self {
        let path = Self::config_path();
        let config = Config::load(&path).unwrap_or_default();
        Self {
            config,
            path,
            page: "general".to_string(),
            cloud_status: CloudStatus::Idle,
        }
    }

    fn update(&mut self, message: Message, context: &ComponentContext<Self>) {
        match message {
            Message::Navigate(Some(tag)) => self.page = tag,
            Message::Navigate(None) => {}

            // 通用页
            Message::LearningLanguage(Some(i)) if i < general::LANGUAGES.len() => {
                self.save("general", "learning_language", general::LANGUAGES[i].1);
            }
            Message::PageSize(Some(value)) => {
                let size = (value.round() as i64).clamp(1, 9);
                self.save("general", "page_size", size);
            }
            Message::Shuangpin(Some(i)) if i < general::SHUANGPIN.len() => {
                self.save("general", "shuangpin", general::SHUANGPIN[i].1);
            }
            Message::EnglishCandidates(on) => self.save("general", "english_candidates", on),
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

            // 云服务页
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
            Message::Translation(Some(i)) if i < shortcut::MODIFIERS.len() => {
                self.save("shortcut", "translation", shortcut::MODIFIERS[i].1);
            }
            Message::TranslationSecond(Some(i)) if i < shortcut::MODIFIERS.len() => {
                self.save("shortcut", "translation_second", shortcut::MODIFIERS[i].1);
            }
            Message::DeleteCandidate(Some(i)) if i < shortcut::MODIFIERS.len() => {
                self.save("shortcut", "delete_candidate", shortcut::MODIFIERS[i].1);
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
                // 用户词库缺省启用，`disabled` 里列的是被关掉的：开=从名单移除，关=加进名单。
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

            // 高级页
            Message::VerboseLog(on) => {
                let level = if on { LogLevel::Debug } else { LogLevel::Info };
                self.save("general", "log_level", level.key());
            }
            Message::InputLog(on) => self.save("general", "input_log", on),
            Message::OpenConfigFile => open_in_editor(&self.path),
            Message::OpenDataDir => {
                open_with_explorer(&self.data_dir().to_string_lossy());
            }
            Message::OpenLogDir => {
                let logs = self.data_dir().join("logs");
                let _ = std::fs::create_dir_all(&logs);
                open_with_explorer(&logs.to_string_lossy());
            }
            Message::ClearInputLog => {
                let log = self.data_dir().join("input-log.jsonl");
                if let Err(error) = std::fs::remove_file(&log)
                    && error.kind() != std::io::ErrorKind::NotFound
                {
                    eprintln!("清空输入日志失败: {error}");
                }
            }

            // 关于页
            Message::OpenWebsite => open_with_explorer(about::WEBSITE_URL),
            Message::OpenRepository => open_with_explorer(about::REPOSITORY_URL),

            // 下拉被清空 / 越界 / 索引已失效：不改。
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
            item("usage", "统计", Symbol::List),
            item("advanced", "高级", Symbol::Repair),
            item("about", "关于", Symbol::Help),
        ];
        NavigationView::new()
            .pane_display_mode(NavigationViewPaneDisplayMode::Left)
            .pane_title("青简")
            .open_pane_length(220.0)
            .is_pane_open(true)
            // 设置窗不需要折叠导航，也没有上一级：去掉三横线折叠按钮和返回箭头。
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
