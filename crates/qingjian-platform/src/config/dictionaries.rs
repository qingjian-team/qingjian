use serde::{Deserialize, Serialize};

/// 配置文件 `[dictionaries]` 分节：附加词库的开关。
///
/// 两类附加词库：随包的领域词库（`.app` 里 `Resources/dicts/`，法律 / 医学 / 地名 …）缺省关闭，列在 `domains` 里的才加载；
/// 用户目录 `dicts/` 下的 `.qj`（自己导入的）文件在就加载，只有列在 `disabled` 里的（按文件名，不含扩展名）跳过；
/// 导入 / 移除就是加 / 删文件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DictionariesConfig {
    /// 打开的随包领域词库（文件名，不含 `.qj`）。
    pub domains: Vec<String>,

    /// 关掉的用户词库（文件名，不含 `.qj`）。
    pub disabled: Vec<String>,
}

/// 缺省打开的随包领域词库：成语四字全拼几乎不歧义，收益稳；其余按需打开。
pub const DEFAULT_DOMAINS: [&str; 1] = ["idioms"];

impl Default for DictionariesConfig {
    fn default() -> Self {
        Self {
            domains: DEFAULT_DOMAINS.iter().map(|s| (*s).to_owned()).collect(),
            disabled: Vec::new(),
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
