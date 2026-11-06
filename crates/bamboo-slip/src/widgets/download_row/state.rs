//! 一个下载项现在的样子，由使用方按自己的下载任务填。

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DownloadState<'a> {
    /// 还没装。
    Available,

    /// 下着：`fraction` 是 0～1 的进度（不知道总大小时给 0），`text` 是进度条下那行字（「正在下载 3 / 8 MB」）。
    Downloading { fraction: f32, text: &'a str },

    /// 下完了在解压、换进去。
    Installing,

    /// 下载失败，`reason` 写在按钮下面。
    Failed { reason: &'a str },

    /// 装好了；`outdated` 是有新版可更新。
    Installed { outdated: bool },
}
