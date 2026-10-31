//! 设置程序的状态：配置文件（唯一事实源）、主题库、当前页。改动一律原地写回配置文件再重读，输入法热加载它。
//! 结构体与构造在这里，各职责的 `impl` 在子模块。

mod app;
mod config;
mod fields;
mod library;
mod phrases;
mod search;
mod shortcuts;
mod transfer;
mod vocabulary;

#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use bamboo_slip::{FamilyId, SvgImage, TextEdit, Waker};
use qingjian_platform::{Appearance, Config};
use qingjian_render::{TextSizes, Theme, ThemeLibrary};

use crate::jobs::cloud_test::CloudTest;
use crate::jobs::import::ImportJob;
use crate::jobs::model_download::ModelDownload;
use crate::jobs::update_check::UpdateCheck;
use crate::library::Library;
use crate::message::{FieldId, Popup, SizeKind};
use crate::page_id::PageId;
use crate::phrase_editor::PhraseEditor;
use crate::search::Search;
use crate::shortcut::Shortcut;
use crate::stats::Stats;
use crate::system::action::Action;
use crate::system::paths;
use crate::view::previews::Previews;
use crate::vocabulary::Vocabulary;

/// 字号步进的范围（点）。
const SIZES: std::ops::RangeInclusive<f32> = 6.0..=72.0;

pub struct Settings {
    path: PathBuf,

    pub config: Config,

    pub themes: ThemeLibrary,

    pub page: PageId,

    /// 系统是深色：设置窗口自己跟它走，候选窗「跟随系统」时也按它。
    pub system_dark: bool,

    pub previews: RefCell<Previews>,

    pub logo: Option<Arc<SvgImage>>,

    /// 侧栏品牌字用的宋体字族。
    pub serif: FamilyId,

    /// 开着的下拉菜单。
    pub menu: Option<Popup>,

    /// 本机字族，第一次打开字体菜单时扫。
    pub fonts: Option<Vec<String>>,

    /// 点过一次、等再点一次确认的动作（清空输入日志）；点了别的就作废。
    pub armed: Option<Action>,

    /// 配置文件读不了时的错误：这时界面按缺省值显示，每页顶上都提示，改好了重读成功就清掉。
    pub config_error: Option<String>,

    /// 做完一件事的那句话和它显示在哪一行下面（[`Action::slot`]、`"cloud"`），点了别的就收掉。
    pub notice: Option<(&'static str, String)>,

    /// 输入框的编辑状态；回车或离开时才存进配置。
    pub fields: HashMap<FieldId, TextEdit>,

    pub cloud_test: CloudTest,

    pub update: UpdateCheck,

    /// 正在录的快捷键。
    pub recording: Option<Shortcut>,

    /// 词库与码表的清单，进词库页、导入、移除后重扫。
    pub library: Library,

    pub import: ImportJob,

    /// 实验功能里正在下的模型。
    pub download: ModelDownload,

    pub translation_download: crate::jobs::offline_download::OfflineDownload,

    /// 点过一次「删除」、等再点一次确认的模型；点了别的就作废。
    pub remove_armed: Option<qingjian_platform::LocalModel>,

    /// 开着的短语编辑卡片。
    pub phrase: Option<PhraseEditor>,

    /// 统计页的数，进统计页时读。
    pub stats: Option<Stats>,

    /// 统计页在「云服务」层。
    pub stats_cloud: bool,

    /// 生词本页的词，进页时读；学习语言关着为 `None`。
    pub vocabulary: Option<Vocabulary>,

    waker: Option<Waker>,

    /// 「关于」页的数据来源展开着。
    pub sources_open: bool,

    pub search: Search,

    /// Windows 的系统 Ctrl + Space 开着；别的平台总是 `false`。打开窗口时读一次，改了再读。
    pub ctrl_space: bool,
}

#[cfg(windows)]
fn system_ctrl_space() -> bool {
    crate::system::hotkey::ctrl_space_enabled()
}

#[cfg(not(windows))]
fn system_ctrl_space() -> bool {
    false
}

impl Settings {
    /// 改系统的 Ctrl + Space，没改成就在切换键下面说一句；勾选状态按系统实际的来。
    fn set_ctrl_space(&mut self, on: bool) {
        #[cfg(windows)]
        if let Err(error) = crate::system::hotkey::set_ctrl_space(on) {
            self.notice = Some(("switch-keys", error));
        }
        #[cfg(not(windows))]
        let _ = on;
        self.ctrl_space = system_ctrl_space();
    }

    pub fn load(path: PathBuf) -> Self {
        // macOS 偏好设置以前把密钥写在配置同目录的 .env 里，读进环境才知道密钥设没设、测试连接也用得上
        let _ = dotenvy::from_path(path.with_file_name(".env"));
        // 下线的取值（含章·知微）改写成现在的；只由设置程序做，输入法进程只读配置
        match Config::rewrite_retired_values(&path) {
            Ok(true) => tracing::info!("配置里已下线的取值改写成现在的"),
            Ok(false) => {}
            Err(error) => tracing::warn!(%error, "改写配置里已下线的取值失败，照旧读"),
        }
        // 配置写坏了也要能打开设置：按缺省值显示、每页顶上提示，不然用户连改回来的地方都没有
        let (config, config_error) = match Config::load(&path) {
            Ok(config) => (config, None),
            Err(error) => {
                tracing::warn!(%error, "配置文件有错误，设置按缺省值打开");
                (Config::default(), Some(error.to_string()))
            }
        };
        let themes = ThemeLibrary::load(paths::themes_dir().as_deref());
        let logo = SvgImage::parse(include_bytes!("../../assets/logo.svg"))
            .map(Arc::new)
            .inspect_err(|error| tracing::warn!(%error, "logo 解析失败"))
            .ok();
        Self {
            path,
            config,
            config_error,
            themes,
            page: PageId::Input,
            system_dark: false,
            previews: RefCell::new(Previews::new()),
            logo,
            serif: FamilyId::default(),
            menu: None,
            fonts: None,
            armed: None,
            notice: None,
            sources_open: false,
            fields: HashMap::new(),
            cloud_test: CloudTest::default(),
            update: UpdateCheck::load(),
            recording: None,
            library: Library::scan(),
            import: ImportJob::default(),
            download: ModelDownload::default(),
            translation_download: crate::jobs::offline_download::OfflineDownload::default(),
            remove_armed: None,
            phrase: None,
            stats: None,
            stats_cloud: false,
            vocabulary: None,
            waker: None,
            search: Search::default(),
            ctrl_space: system_ctrl_space(),
        }
        .with_fields()
    }

    /// 候选窗该用深色：按「外观」设置，跟随系统时看系统。
    pub fn candidate_dark(&self) -> bool {
        match self.config.general.appearance() {
            Appearance::System => self.system_dark,
            Appearance::Light => false,
            Appearance::Dark => true,
        }
    }

    /// 当前主题，按外观与设置里的字号换好。
    pub fn theme(&self, id: &str) -> Theme {
        let general = &self.config.general;
        self.themes
            .resolve(id, self.candidate_dark())
            .with_text_sizes(TextSizes::new(
                general.candidate_font_size.0,
                general.annotation_font_size.0,
            ))
    }

    /// 字号框里显示的数：设置里写了用它，没写用主题的。
    pub fn size(&self, kind: SizeKind) -> f32 {
        let general = &self.config.general;
        let (own, name) = match kind {
            SizeKind::Candidate => (general.candidate_font_size, "candidate"),
            SizeKind::Annotation => (general.annotation_font_size, "annotation"),
        };
        own.get()
            .or_else(|| self.theme(general.theme_id()).theme_size(name))
            .unwrap_or(13.0)
    }
}
