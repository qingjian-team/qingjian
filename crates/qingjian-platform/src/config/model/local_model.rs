use serde::{Deserialize, Serialize};

/// 用哪个本地模型，三个只开一个。通变随安装包；知微、知意要在设置的「实验功能」里下载。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LocalModel {
    /// 含章·通变：按敲的拼音给整句重排，不读前文。
    #[default]
    Tongbian,

    /// 含章·知微：读前文给整句重排。
    Zhiwei,

    /// 知意：读前文与敲的拼音，从候选窗前几条里挑首选。
    Zhiyi,
}

impl LocalModel {
    /// 配置文件里的写法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Tongbian => "tongbian",
            Self::Zhiwei => "zhiwei",
            Self::Zhiyi => "zhiyi",
        }
    }

    /// 界面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Tongbian => "含章·通变",
            Self::Zhiwei => "含章·知微",
            Self::Zhiyi => "知意",
        }
    }
}
