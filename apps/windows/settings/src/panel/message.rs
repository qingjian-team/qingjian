//! 设置窗口的消息类型：导航切换与各页的「改动」，根组件的 `update` 据此落盘。

/// 设置窗口的消息；「改动」消息带控件新值，`update` 据此落盘。
#[derive(Clone)]
pub(crate) enum Message {
    /// 导航切换分节（`None` 是取消选中，忽略）。
    Navigate(Option<String>),

    // 通用页
    LearningLanguage(Option<usize>),
    PageSize(Option<f64>),
    Scheme(Option<usize>),
    ShuangpinRawPreedit(bool),
    Wubi(bool),
    Traditional(bool),
    EnglishCandidates(bool),
    ChineseFirst(bool),
    /// 中文模式下 Shift+字母：交给应用（缺省）还是进组句缓冲区。
    ShiftLetter(Option<usize>),
    FullWidthPunctuation(bool),
    EnglishFullWidthPunctuation(bool),
    /// 开=写入平台默认名单，关=清空。
    EnglishOffInApps(bool),
    /// 中英切换键（[`qingjian_platform::SwitchKey`] 的下标）。
    SwitchMode(Option<usize>),
    /// 内置英文模式总开关。
    EnglishMode(bool),

    // 候选窗口页
    Theme(Option<usize>),
    Layout(Option<usize>),
    Preedit(Option<usize>),
    Renderer(Option<usize>),
    /// 字体框里的文字变了：空或正好是某个字族名就落盘。
    FontQuery(String),
    /// 从提示里选了一个字族。
    Font(String),
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
    QuestionMark(bool),
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

    // 辅码页
    /// 辅码总开关（`[aux_code] enabled`，缺省关）。
    AuxCodeEnabled(bool),
    /// 候选上是否显示码。
    AuxCodeShow(bool),
    /// 码删空后是否留在辅码态（`[general] aux_code_keep_empty`）。
    AuxCodeKeepEmpty(bool),
    /// 点「录制」：进入等一个键的状态。
    AuxRecordStart,
    /// 录制中放弃，保持原值。
    AuxRecordCancel,
    /// 录制框敲进来的文本，取第一个字符当新触发键。
    AuxRecorded(String),
    /// 码表开关（名字，开 / 关），关掉的进 `[aux_code] disabled`。
    ToggleAuxTable(String, bool),
    /// 挪进 codes\removed，不真删。
    RemoveAuxTable(String),
    /// 打开文件选择器导入一张码表（Rime `.dict.yaml`）。
    ImportCodeTable,

    // 高级页
    VerboseLog(bool),
    InputLog(bool),
    /// 学习输入习惯开关。
    Learning(bool),
    OpenConfigFile,
    OpenDataDir,
    OpenLogDir,
    /// 日志目录 + config.toml 打成 zip 放桌面。
    ExportLogs,
    ClearInputLog,

    // 关于页
    OpenWebsite,
    OpenRepository,
}
