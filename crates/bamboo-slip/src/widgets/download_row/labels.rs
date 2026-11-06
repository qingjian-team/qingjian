//! 下载项按钮上的字：引擎不带文案，由使用方按界面语言给。

#[derive(Debug, Clone, Copy)]
pub struct DownloadLabels<'a> {
    pub download: &'a str,

    pub cancel: &'a str,

    pub installing: &'a str,

    pub retry: &'a str,

    pub delete: &'a str,

    /// 点过一次「删除」后按钮上的字（「再点一次删除」）。
    pub delete_confirm: &'a str,

    pub update: &'a str,

    /// 这一版程序用不了它时灰着的按钮上的字（「要先更新」）。
    pub blocked: &'a str,
}
