//! 导入词库、码表：选文件在界面线程（系统的文件选择窗口要在主线程开），转换放后台线程（大的 Rime 词库要转好几秒），
//! 转完叫醒窗口。词库只有一个入口：第二列是形码编码的（五笔词库）自动改存成形码词库，跟方案的码表合用。

mod report;
mod target;

pub use report::Report;
pub use target::CodeTarget;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use bamboo_slip::Waker;
use qingjian_dictionary::import::{ImportedDictionary, import_code_words};
use qingjian_dictionary::{Dictionary, DictionaryError, import, import_aux_code_table};

use crate::library::Kind;

/// 导入结果里每类跳过的词最多列几个。
const MAX_SKIPPED_SHOWN: usize = 5;

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
    pub fn start(
        &mut self,
        kind: Kind,
        code: CodeTarget,
        waker: Option<Waker>,
    ) -> Result<bool, String> {
        let dir = kind.user_dir().ok_or("找不到数据目录")?;
        let words_dir = Kind::CodeWords.user_dir().ok_or("找不到数据目录")?;
        let dialog = match kind {
            Kind::Dictionary | Kind::CodeWords => rfd::FileDialog::new()
                .add_filter("词库文件", &["tsv", "txt", "yaml", "yml", "qj"])
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
            let report = run(kind, &sources, &dir, &words_dir, &code);
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

fn run(kind: Kind, sources: &[PathBuf], dir: &Path, words_dir: &Path, code: &CodeTarget) -> Report {
    let mut imported = Vec::new();
    let mut lines = Vec::new();
    // 无读音词表按随包词库派生读音：只在词库这一路用得上，也只在这一路读一次
    let references = (kind == Kind::Dictionary).then(crate::library::reference_dictionaries);
    let borrowed: Vec<&Dictionary> = references.iter().flatten().collect();
    for source in sources {
        let file = source.file_name().map_or_else(
            || source.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        let outcome = match kind {
            Kind::Dictionary => match import::import_dictionary(source, dir, &borrowed) {
                Err(DictionaryError::CodeWords) => import_code_words(source, words_dir, &code.id)
                    .map(|done| {
                        (
                            done.path,
                            format!(
                                "已导入「{}」，{} 条，并进{}的码表",
                                done.name, done.entries, code.label
                            ),
                        )
                    }),
                other => other.map(|done| {
                    let line = dictionary_line(&done);
                    (done.imported.path, line)
                }),
            },
            Kind::CodeWords => match import::import(source, dir) {
                Err(DictionaryError::CodeWords) => import_code_words(source, words_dir, &code.id)
                    .map(|done| {
                        (
                            done.path,
                            format!(
                                "已导入「{}」，{} 条，并进{}的码表",
                                done.name, done.entries, code.label
                            ),
                        )
                    }),
                imported => imported.map(|done| {
                    (
                        done.path,
                        format!("已导入「{}」，{} 条", done.name, done.entries),
                    )
                }),
            },
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
            Err(DictionaryError::NoWordEntries) => {
                lines.push(format!("{file}：词表里的字都不在随包词库，推不出读音"));
            }
            Err(DictionaryError::Corrupt(_) | DictionaryError::Line { .. })
                if kind != Kind::CodeTable =>
            {
                tracing::warn!(source = %source.display(), "导入失败，格式不对");
                lines.push(format!(
                    "{file}：没认出格式。词库要「词、拼音、词频」三列，拼音音节用空格分开；只有词的表（txt 或 Rime 词库）也行，读音按随包词库补；五笔词库要「词、编码、词频」三列"
                ));
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

/// 词库导入的结果行：无读音词表附上跳过的条数与几个词，普通词库只写条数。
fn dictionary_line(done: &ImportedDictionary) -> String {
    let imported = &done.imported;
    let mut line = format!("已导入「{}」，{} 条", imported.name, imported.entries);
    let Some(report) = &done.report else {
        return line;
    };
    if report.unknown.is_empty() {
        return line;
    }
    let shown: Vec<&str> = report
        .unknown
        .iter()
        .take(MAX_SKIPPED_SHOWN)
        .map(String::as_str)
        .collect();
    line.push_str(&format!(
        "；{} 条跳过（字不在随包词库）：{}{}",
        report.unknown.len(),
        shown.join("、"),
        if report.unknown.len() > MAX_SKIPPED_SHOWN {
            "…"
        } else {
            ""
        }
    ));
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use qingjian_dictionary::import::{Imported, Report};

    fn done(report: Option<Report>) -> ImportedDictionary {
        ImportedDictionary {
            imported: Imported {
                path: PathBuf::from("dicts/names.qj"),
                name: "names".to_owned(),
                entries: 12,
            },
            report,
        }
    }

    #[test]
    fn word_list_results_mention_the_skipped_words() {
        assert_eq!(dictionary_line(&done(None)), "已导入「names」，12 条");
        let known = Report {
            words: 12,
            unknown: Vec::new(),
        };
        assert_eq!(
            dictionary_line(&done(Some(known))),
            "已导入「names」，12 条"
        );
        let unknown = ["囧一", "囧二", "囧三", "囧四", "囧五", "囧六"]
            .map(str::to_owned)
            .to_vec();
        assert_eq!(
            dictionary_line(&done(Some(Report { words: 12, unknown }))),
            "已导入「names」，12 条；6 条跳过（字不在随包词库）：囧一、囧二、囧三、囧四、囧五…"
        );
    }
}
