//! 词库导入完成后的收尾，升级后补下词库。

use super::Settings;
use qingjian_platform::code_schemes::WUBI86;

use crate::i18n::{tr, tr_args};
use crate::jobs::download::Item;
use crate::jobs::import::{CodeTarget, Report};
use crate::library::{self, Kind, Library};

impl Settings {
    /// 导入的形码词库记给哪套方案：现在选的形码方案，没开形码就是 86 五笔。
    pub(super) fn code_target(&self) -> CodeTarget {
        let id = self
            .config
            .general
            .code()
            .unwrap_or_else(|| WUBI86.to_owned());
        CodeTarget {
            label: self.library.code_scheme_label(&id),
            id,
        }
    }

    /// 导入完了：之前关掉过的同名词库重新打开（码表改成用刚导入的那张），重扫清单，把结果显示出来。
    pub(super) fn imported(&mut self, report: Report) {
        let kind = report.kind;
        let (section, key) = kind.disabled_key();
        let disabled = match kind {
            Kind::Dictionary | Kind::CodeWords => &self.config.dictionaries.disabled,
            Kind::CodeTable => &self.config.aux_code.disabled,
        };
        let kept: Vec<String> = disabled
            .iter()
            .filter(|stem| !report.imported.contains(stem))
            .cloned()
            .collect();
        if kept.len() != disabled.len() {
            self.write_list(section, key, &kept);
        }
        if kind == Kind::CodeTable
            && let Some(stem) = report.imported.last()
        {
            self.write("aux_code", "scheme", stem.as_str());
        }
        self.library = Library::scan();
        self.notice = Some((kind.slot(), report.lines.join("\n")));
    }

    /// 把「我的词库」里的一本导出成 TSV：选存到哪，结果显示在词库那一组下面。
    pub(super) fn export_dict(&mut self, stem: &str) {
        let Some(target) = rfd::FileDialog::new()
            .set_title(tr("library.export_title"))
            .set_file_name(format!("{stem}.tsv"))
            .add_filter(tr("library.tsv_filter"), &["tsv"])
            .save_file()
        else {
            return;
        };
        let notice = library::export_tsv(stem, &target).map(|count| {
            tr_args(
                "library.exported",
                &[("n", &count), ("path", &target.display())],
            )
        });
        self.notice = Some((
            Kind::Dictionary.slot(),
            notice.unwrap_or_else(|error| error),
        ));
    }

    /// 升级后要补下的词库全部排进下载队列（已经在下、在排的不重复加）。词库清单到手时调；
    /// 失败的这次不重试，下次打开设置再排。
    pub(super) fn restore_next(&mut self) {
        let index = self.dict_index.state().dicts;
        for entry in self.restore.pending(&self.config, &self.library, &index) {
            if self
                .download
                .stage_of(&Item::Dict(entry.id.clone()))
                .is_some()
            {
                continue;
            }
            tracing::info!(id = entry.id, "restoring dictionary after upgrade");
            if let Err(error) = self.download.start(entry.target(), self.waker.clone()) {
                tracing::warn!(id = entry.id, error, "restore download not started");
            }
        }
    }
}
