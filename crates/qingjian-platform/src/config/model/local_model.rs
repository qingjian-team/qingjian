use serde::{Deserialize, Serialize};

use crate::i18n::tr;

/// 用哪个本地模型，两个只开一个。都在设置的「实验功能」里下载（Linux 随包带通变）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LocalModel {
    /// 含章·通变：按敲的拼音直接生成整句（造句、造词），够长的句子以它为主，不读前文。
    /// 含章·知微已下线（同是看前文排序，知意强得多），老配置里的 `zhiwei` 按通变读。
    #[default]
    #[serde(alias = "zhiwei")]
    Tongbian,

    /// 知意：读前文与敲的拼音，从候选窗前几条里挑首选。
    Zhiyi,
}

impl LocalModel {
    /// 配置文件里的写法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Tongbian => "tongbian",
            Self::Zhiyi => "zhiyi",
        }
    }

    /// 界面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Tongbian => tr("model.tongbian"),
            Self::Zhiyi => tr("model.zhiyi"),
        }
    }
}
