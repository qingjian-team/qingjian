//! 导入词库、码表：选文件在界面线程（系统的文件选择窗口要在主线程开），转换放后台线程（大的 Rime 词库要转好几秒），
//! 转完叫醒窗口。

mod report;

pub use report::Report;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use bamboo_slip::Waker;
use qingjian_dictionary::{DictionaryError, import, import_aux_code_table};

use crate::library::Kind;

#[derive(Default)]
pub struct ImportJob {
    result: Arc<Mutex<Option<Report>>>,

    running: Option<Kind>,
}

impl ImportJob {
    pub fn running(&self) -> Option<Kind> {
        self.running
    }

    /// 弹出文件选择窗口；没选就算了。
    pub fn start(&mut self, kind: Kind, waker: Option<Waker>) -> Result<bool, String> {
        let dir = kind.user_dir().ok_or("找不到数据目录")?;
        let dialog = match kind {
            Kind::Dictionary => rfd::FileDialog::new()
                .add_filter("词库文件", &["tsv", "yaml", "yml", "qj"])
                .set_title("导入词库"),
            Kind::CodeTable => rfd::FileDialog::new()
                .add_filter("码表文件", &["yaml", "yml", "qj"])
                .set_title("导入码表"),
        };
        let Some(sources) = dialog.add_filter("所有文件", &["*"]).pick_files() else {
            return Ok(false);
        };
        self.running = Some(kind);
        let result = Arc::clone(&self.result);
        std::thread::spawn(move || {
            let report = run(kind, &sources, &dir);
            if let Ok(mut slot) = result.lock() {
                *slot = Some(report);
            }
            if let Some(waker) = waker {
                waker.wake();
            }
        });
        Ok(true)
    }

    pub fn finish(&mut self) -> Option<Report> {
        let report = self.result.lock().ok()?.take()?;
        self.running = None;
        Some(report)
    }
}

fn run(kind: Kind, sources: &[PathBuf], dir: &Path) -> Report {
    let mut imported = Vec::new();
    let mut lines = Vec::new();
    for source in sources {
        let file = source.file_name().map_or_else(
            || source.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        let outcome = match kind {
            Kind::Dictionary => import::import(source, dir).map(|done| {
                (
                    done.path,
                    format!("已导入「{}」，{} 条", done.name, done.entries),
                )
            }),
            Kind::CodeTable => import_aux_code_table(source, dir).map(|done| {
                let report = done.report;
                (
                    done.path,
                    format!(
                        "已导入「{}」：读入 {}，有码 {}，无码 {}，跳过 {}",
                        done.name, report.read, report.with_code, report.no_code, report.skipped
                    ),
                )
            }),
        };
        match outcome {
            Ok((path, line)) => {
                if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
                    imported.push(stem.to_owned());
                }
                lines.push(line);
            }
            Err(DictionaryError::NoCodeEntries) => {
                lines.push(format!("{file}：没认出码表，要「词 + 码」两列，码只收 a–z"));
            }
            Err(error) => {
                tracing::warn!(source = %source.display(), %error, "导入失败");
                lines.push(format!("{file} 导入失败：{error}"));
            }
        }
    }
    Report {
        kind,
        imported,
        lines,
    }
}
