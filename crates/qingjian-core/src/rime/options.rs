//! 运行库和数据目录必须由用户显式提供，不下载或捆绑第三方词库。
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RimeOptions {
    /// 与当前进程位数一致、包含 librime-lua 的动态库。
    pub library: PathBuf,

    /// 雾凇原始文件及 Rime 公共配置、OpenCC 数据所在目录。
    pub shared_data: PathBuf,

    /// 可写的独立用户目录；保存 .custom.yaml、编译文件和用户词频。
    pub user_data: PathBuf,

    /// 初始方案标识；之后可用 Rime 的方案菜单切换。
    pub schema: String,

    /// 静态注册在所选运行库中的额外插件模块，default 与 lua 始终启用。
    pub modules: Vec<String>,
}
