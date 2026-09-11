//! 设置窗口的消息类型：导航切换与各页的「改动」，根组件的 `update` 据此落盘。

/// 设置窗口的消息；「改动」消息带控件新值，`update` 据此落盘。
#[derive(Clone)]
pub(crate) enum Message {
    /// 导航切换分节（`None` 是取消选中，忽略）。
    Navigate(Option<String>),

    // 通用页
    LearningLanguage(Option<usize>),
    PageSize(Option<f64>),
    Shuangpin(Option<usize>),
    EnglishCandidates(bool),
    FullWidthPunctuation(bool),
    EnglishFullWidthPunctuation(bool),
    /// 开=写入平台默认名单，关=清空。
    EnglishOffInApps(bool),

    // 候选窗口页
    Theme(Option<usize>),
    Layout(Option<usize>),
    Preedit(Option<usize>),
    StatusBar(bool),

    // 云服务页
    LocalModel(bool),
    CloudEnabled(bool),
    CloudApiKey(String),
    CloudModel(String),
    CloudBaseUrl(String),
    CloudSlots(Option<f64>),
    CloudSentence(bool),
    TestConnection,
    CloudTestDone(Result<String, String>),

    // 快捷键页
    PageKeys(Option<usize>),
    ModeExpression(Option<usize>),
    ModeQuestion(Option<usize>),
    Translation(Option<usize>),
    TranslationSecond(Option<usize>),
    DeleteCandidate(Option<usize>),
    /// 只换修饰键，字母键固定用当前的。
    TranslateSelection(Option<usize>),

    // 模糊音页
    /// 配置键 + 新值。
    Fuzzy(&'static str, bool),

    // 词库页
    ToggleDomain(String, bool),
    ToggleUserDict(String, bool),
    /// 挪进 dicts\removed，不真删。
    RemoveUserDict(String),
    ImportDictionary,

    // 高级页
    VerboseLog(bool),
    InputLog(bool),
    OpenConfigFile,
    OpenDataDir,
    OpenLogDir,
    ClearInputLog,

    // 关于页
    OpenWebsite,
    OpenRepository,
}
