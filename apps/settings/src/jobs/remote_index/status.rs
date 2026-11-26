//! 这次取清单走到哪了。

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Status {
    /// 还没开始取（设置窗口刚建、还没拿到叫醒的句柄）。
    #[default]
    Idle,

    Loading,

    /// 取到了，已存进缓存。
    Fetched,

    /// 没取到；手上有缓存时照样显示缓存的那份。
    Failed(String),
}
