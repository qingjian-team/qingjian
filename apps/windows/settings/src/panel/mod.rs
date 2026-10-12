//! 设置窗口根组件：左侧导航 + 右侧当前分节页。每改一项就原地写回 `config.toml`（保留注释）再重读，
//! 界面始终反映文件内容；Server 每秒看 mtime 热加载。
//! 状态在这里，消息在 [`message`]，生命周期在 [`component`]，表单零件在 [`controls`]，各页在 [`pages`]。

mod cloud_status;
mod component;
mod controls;
mod message;
mod notice;
mod pages;
mod recorder;

use std::path::{Path, PathBuf};

use qingjian_platform::Config;
use qingjian_render::ThemeLibrary;
use windows_reactor::*;

use self::cloud_status::CloudStatus;
pub(crate) use self::message::Message;
use self::notice::Notice;
use self::pages::{
    about, advanced, aux_code, candidates, cloud, dictionaries, fuzzy, general, shortcut, usage,
};
use self::recorder::Recorder;

/// 左侧标签固定宽度，让各行控件对齐。
const LABEL_WIDTH: f64 = 140.0;

/// 设置窗口状态。
pub(crate) struct Settings {
    /// 当前配置，每次改动后从盘上重读。
    pub(super) config: Config,

    /// `config.toml` 路径。
    path: PathBuf,

    /// 当前导航分节 tag。
    page: String,

    /// 云服务「测试连接」的状态。
    cloud_status: CloudStatus,

    /// 「辅码」页的触发键录制状态。
    recorder: Recorder,

    /// 触发键录制框（密码框：不走输入法，按 A–Z 直接进字符、不弹输入法候选窗）：
    /// 进了录制态把焦点交给它，用户不用再点一下。
    record_box: ElementRef<PasswordBox>,

    /// 页面底部的临时提示（导入统计 / 失败原因）。
    notice: Notice,

    /// Server 或「立即检查」落盘的检查更新结果（用户目录的 `update.json`）。
    update_state: qingjian_update::UpdateState,

    /// 「立即检查」正在跑 / 刚失败的原因。
    update_checking: bool,
    update_error: Option<String>,

    /// 最近一次词库操作的结果，显示在词库页。
    dictionary_status: String,

    /// 系统里的字族名（DirectWrite），「字体」框的提示用。
    families: Vec<String>,

    /// 「字体」框里正在敲的文字；`None` 显示配置里的值。
    font_query: Option<String>,
}

impl Settings {
    /// `%APPDATA%\Qingjian\config.toml`；取不到 `APPDATA` 退回工作目录。
    fn config_path() -> PathBuf {
        qingjian_platform::dirs::config_path().unwrap_or_else(|| PathBuf::from("config.toml"))
    }

    /// 检查更新的结果文件 `%APPDATA%\Qingjian\update.json`（Server 写，这里读）。
    fn update_state_path() -> Option<PathBuf> {
        qingjian_platform::dirs::user_dir().map(|dir| dir.join("update.json"))
    }

    /// 配置文件不在就写出模板：这个账户下 Server 还没跑过时，保存与「在记事本中打开」都要有文件。
    fn ensure_config_file(path: &Path) {
        if let Err(error) = Config::write_template_if_missing(path) {
            crate::log::warn(format!("写配置模板失败: {error}"));
        }
    }

    /// 数据目录 `%APPDATA%\Qingjian`。
    fn data_dir(&self) -> &Path {
        self.path.parent().unwrap_or_else(|| Path::new("."))
    }

    /// 落盘一个配置值再重读。失败只打印。
    fn save(&mut self, section: &str, key: &str, value: impl Into<toml_edit::Value>) {
        if let Err(error) = Config::set_value(&self.path, section, key, value) {
            crate::log::warn(format!("保存 [{section}] {key} 失败: {error}"));
            return;
        }
        self.reload();
    }

    /// 字号框（`style` 是 `candidate` / `annotation`）：清空或填 0 回到主题的，不限范围；
    /// 没设过时框里显示主题的字号，原样交回不写（见 [`qingjian_platform::FontSize::edited`]）。
    fn save_font_size(&mut self, style: &str, size: Option<f64>) {
        let current = match style {
            "candidate" => self.config.general.candidate_font_size,
            _ => self.config.general.annotation_font_size,
        };
        let themes = ThemeLibrary::load(qingjian_platform::dirs::themes_dir().as_deref());
        let theme_size = themes
            .resolve(self.config.general.theme_id(), false)
            .theme_size(style);
        if let Some(size) = current.edited(size.filter(|size| !size.is_nan()), theme_size) {
            self.save("general", &format!("{style}_font_size"), size);
        }
    }

    /// 落盘一个字符串数组再重读。
    fn save_array(&mut self, section: &str, key: &str, values: &[String]) {
        if let Err(error) = Config::set_array(&self.path, section, key, values) {
            crate::log::warn(format!("保存 [{section}] {key} 失败: {error}"));
            return;
        }
        self.reload();
    }

    fn reload(&mut self) {
        if let Ok(config) = Config::load(&self.path) {
            self.config = config;
        }
    }

    fn page_content(&self, context: &mut ViewContext<Self>) -> View {
        match self.page.as_str() {
            "candidates" => candidates::view(self, context),
            "shortcut" => shortcut::view(self, context),
            "cloud" => cloud::view(self, context),
            "fuzzy" => fuzzy::view(self, context),
            "dictionaries" => dictionaries::view(self, context),
            "aux_code" => aux_code::view(self, context),
            "usage" => usage::view(self, context),
            "advanced" => advanced::view(self, context),
            "about" => about::view(self, context),
            _ => general::view(self, context),
        }
    }
}
