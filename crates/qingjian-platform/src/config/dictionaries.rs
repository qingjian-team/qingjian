use serde::{Deserialize, Serialize};

/// 配置文件 `[dictionaries]` 分节：附加词库的开关。
///
/// 两类附加词库：随包的领域词库（`.app` 里 `Resources/dicts/`，法律 / 医学 / 地名 …）缺省关闭，列在 `domains` 里的才加载；
/// 用户目录 `dicts/` 下的 `.qj`（自己导入的）文件在就加载，只有列在 `disabled` 里的（按文件名，不含扩展名）跳过；
/// 导入 / 移除就是加 / 删文件。随包基础词库单独一项 `base`，不和文件名混在 `disabled` 里（用户词库可能也叫 base）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DictionariesConfig {
    /// 打开的随包领域词库（文件名，不含 `.qj`）。
    pub domains: Vec<String>,

    /// 随包基础词库里的词：`false` 时只出单字（`Engine::set_base_words`），词全靠别的词库。
    pub base: bool,

    /// 关掉的用户词库（文件名，不含 `.qj`）。
    pub disabled: Vec<String>,

    /// 屏蔽词：候选里**含有**这些词的整条都不出（`搜狗` 也挡掉 `搜狗拼音`、`搜狗输入法`）。
    ///
    /// 用在导入的词库带进来一堆用不上的品牌词时——某些第三方词库里「搜狗」这类词频被填得很高，
    /// 打 `sougo` 这种错拼都能顶到第一页。自定义短语不受影响：那是用户自己写下的原文。
    #[serde(default)]
    pub blocked: Vec<String>,
}

/// 缺省打开的随包领域词库：成语四字全拼几乎不歧义，收益稳；其余按需打开。
pub const DEFAULT_DOMAINS: [&str; 1] = ["idioms"];

impl Default for DictionariesConfig {
    fn default() -> Self {
        Self {
            domains: DEFAULT_DOMAINS.iter().map(|s| (*s).to_owned()).collect(),
            base: true,
            disabled: Vec::new(),
            blocked: Vec::new(),
        }
    }
}

impl DictionariesConfig {
    /// 用户目录里的词库是否启用。
    pub fn is_enabled(&self, stem: &str) -> bool {
        !self.disabled.iter().any(|d| d == stem)
    }

    /// 随包领域词库是否启用。
    pub fn is_domain_enabled(&self, stem: &str) -> bool {
        self.domains.iter().any(|d| d == stem)
    }
}
