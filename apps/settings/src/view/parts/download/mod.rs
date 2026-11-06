//! 下载项（模型、词库、释义表）画在 bamboo-slip 的 [`bamboo_slip::DownloadRow`] 上，这里只把设置程序的东西翻过去：
//! 下载任务的状态（[`DownloadStatus`]）、按界面语言的按钮文字（[`labels`]），以及大小的写法。

mod status;

pub use status::DownloadStatus;

use bamboo_slip::DownloadLabels;

use crate::i18n::tr;

pub fn labels() -> DownloadLabels<'static> {
    DownloadLabels {
        download: tr("experimental.download"),
        cancel: tr("experimental.cancel"),
        installing: tr("experimental.installing"),
        retry: tr("experimental.retry"),
        delete: tr("experimental.delete"),
        delete_confirm: tr("experimental.delete_confirm"),
        update: tr("dictionaries.update"),
        blocked: tr("dictionaries.needs_newer_app"),
    }
}

/// 下载进度与词库大小的写法：10 MB 以下留一位小数（领域词库多半不到 1 MB）。
pub fn megabytes(bytes: u64) -> String {
    let megabytes = bytes as f64 / 1_000_000.0;
    if megabytes < 10.0 {
        format!("{megabytes:.1} MB")
    } else {
        format!("{megabytes:.0} MB")
    }
}
