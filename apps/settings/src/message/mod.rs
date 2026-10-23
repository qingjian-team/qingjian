//! 控件产出的消息：多数是「把配置文件里的某一项改成某个值」，其余是界面自己的状态与按钮动作。

mod field_id;
mod popup;
mod size_kind;

pub use field_id::FieldId;
pub use popup::Popup;
pub use size_kind::SizeKind;

use bamboo_slip::{Edit, KeyPress};
use qingjian_core::Language;

use crate::library::Kind;
use crate::page_id::PageId;
use crate::shortcut::Shortcut;
use crate::system::action::Action;
use crate::vocabulary::{Filter, Format, Sort, View};

#[derive(Debug, Clone)]
pub enum Message {
    Page(PageId),
    /// 原地改配置文件里的 `[分节] 键 = 值`。
    Set(&'static str, &'static str, toml_edit::Value),
    /// 同上，值是字符串数组。
    SetList(&'static str, &'static str, Vec<String>),
    /// 改当前形码方案的一条按键规则（`[code.<id>] 键 = 值`，id 是 `[general] code` 选的那套）。
    SetCodeRule(&'static str, bool),
    OpenMenu(Popup),
    /// 关掉弹出菜单（点了菜单外面、按了 Esc）。
    CloseMenu,
    Size(SizeKind, i32),
    ResetSizes,
    Run(Action),
    /// 输入框里的一次编辑（打字、挪光标、输入法组字）。
    Edit(FieldId, Edit),
    /// 回车或离开输入框：存进配置。
    Commit(FieldId),
    /// Esc：扔掉没存的改动。
    Cancel(FieldId),
    TestCloud,
    /// 开始录这个快捷键。
    Record(Shortcut),
    /// 录制中按下的键。
    Recorded(Shortcut, KeyPress),
    /// Esc 或点了别处：不录了。
    StopRecording,
    /// 快捷键页的「恢复默认」。
    ResetShortcuts,
    /// 打开短语编辑卡片：`None` 新增，`Some(i)` 改第 i 条。
    OpenPhrase(Option<usize>),
    SavePhrase,
    /// 卡片里的「删除」（第一下只是确认）。
    DeletePhrase,
    ClosePhrase,
    /// 列表里直接开关第 i 条。
    TogglePhrase(usize),
    /// 选文件导入词库或码表。
    Import(Kind),
    /// 移除用户导入的一本（文件名不含扩展名）。
    Remove(Kind, String),
    /// 勾掉或勾上 Windows 的系统 Ctrl + Space（改的是系统设置，不是配置文件）。
    SystemCtrlSpace(bool),
    /// 表达式、问字两个模式键之一换成这个字母（撞了就对调）。
    ModeKey(&'static str, char),
    CheckUpdate,
    /// 实验功能里下载这个模型（失败后的「重试」也是它）。
    DownloadModel(qingjian_platform::LocalModel),
    CancelDownload,
    /// 中英双向离线翻译模型，独立于整句排序模型。
    DownloadTranslation,
    CancelTranslationDownload,
    /// 删掉下好的模型（第一下只是确认）。
    RemoveModel(qingjian_platform::LocalModel),
    /// 展开 / 收起「关于」页的数据来源。
    ToggleSources,
    /// 统计页切到「云服务」层（`false` 回「输入」层）。
    StatsCloud(bool),
    /// 搜索框里打字。
    Search(Edit),
    /// 搜索框里回车：跳到第一条结果。
    SearchFirst,
    /// 搜索框里 Esc：清空，回到导航。
    ClearSearch,
    /// 点了第几条搜索结果。
    Jump(usize),
    /// 生词本切「时间线 / 词表」。
    VocabularyView(View),
    /// 生词本切到这门语言。
    VocabularyLanguage(Language),
    /// 时间线翻到这天。
    VocabularyDay(jiff::civil::Date),
    VocabularyFilter(Filter),
    VocabularySort(Sort),
    /// 生词本翻到第几页（从 0 起）。
    VocabularyPage(usize),
    /// 标这个词认识（`false` 取消）。
    VocabularyKnown(String, bool),
    VocabularyRemove(String),
    /// 清空生词本（第一下只是确认）。
    VocabularyClear,
    VocabularyExport(Format),
    /// 点了 ⓘ：打开这个网址（官网文档的一段）。
    OpenDoc(String),
    Quit,
}

impl Message {
    pub fn set(
        section: &'static str,
        key: &'static str,
        value: impl Into<toml_edit::Value>,
    ) -> Self {
        Self::Set(section, key, value.into())
    }
}
