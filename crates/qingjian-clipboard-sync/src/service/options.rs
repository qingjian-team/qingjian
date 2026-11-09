//! [`SyncOptions`]：启动同步服务要给的东西。

use std::path::PathBuf;

pub struct SyncOptions {
    /// 身份和配对名单放的目录。
    pub directory: PathBuf,

    /// 给别的设备看的名字。
    pub name: String,

    /// 型号或电脑名，显示在名字旁边。
    pub model: String,

    /// 自己用 mDNS 公布和发现。安卓关掉，由系统的 NsdManager 代劳，结果经 [`super::SyncService::report`] 交进来。
    pub own_discovery: bool,
}
