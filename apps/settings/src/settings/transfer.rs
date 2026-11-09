//! 「高级」页的设置导入导出：只搬设置，短语、云服务密钥与状态条位置留在这台电脑（规则在
//! `qingjian_platform::export_settings` / `import_settings`）。导入前把现在的配置另存一份。

use qingjian_platform::{export_settings, import_settings};

use super::Settings;

use crate::i18n::{tr, tr_args};

/// 消息挂在哪一行下面。
const SLOT: &str = "config-transfer";

/// 导入前现在的配置另存成这个名字（与配置同目录），导错了能拷回来。
const BEFORE_IMPORT: &str = "config.before-import.toml";

impl Settings {
    pub(super) fn export_config(&mut self) {
        let source = match std::fs::read_to_string(&self.path) {
            Ok(source) => source,
            Err(error) => {
                self.notice = Some((
                    SLOT,
                    tr_args("transfer.cant_read_config", &[("error", &error)]),
                ));
                return;
            }
        };
        let exported = match export_settings(&source) {
            Ok(exported) => exported,
            Err(error) => {
                self.notice = Some((SLOT, error));
                return;
            }
        };
        let name = tr_args(
            "transfer.file_name",
            &[("date", &jiff::Zoned::now().date())],
        );
        let Some(target) = rfd::FileDialog::new()
            .set_title(tr("transfer.export_title"))
            .set_file_name(&name)
            .add_filter(tr("transfer.file_filter"), &["toml"])
            .save_file()
        else {
            return;
        };
        let notice = match std::fs::write(&target, exported) {
            Ok(()) => tr_args("transfer.exported", &[("path", &target.display())]),
            Err(error) => tr_args("transfer.cant_write", &[("error", &error)]),
        };
        self.notice = Some((SLOT, notice));
    }

    pub(super) fn import_config(&mut self) {
        let Some(source) = rfd::FileDialog::new()
            .set_title(tr("transfer.import_title"))
            .add_filter(tr("transfer.file_filter"), &["toml"])
            .add_filter(tr("import.all_files"), &["*"])
            .pick_file()
        else {
            return;
        };
        let current = std::fs::read_to_string(&self.path).unwrap_or_default();
        let result = std::fs::read_to_string(&source)
            .map_err(|error| tr_args("transfer.cant_read", &[("error", &error)]))
            .and_then(|imported| import_settings(&current, &imported))
            .and_then(|merged| {
                let backup = self.path.with_file_name(BEFORE_IMPORT);
                if !current.is_empty() {
                    std::fs::write(&backup, &current)
                        .map_err(|error| tr_args("transfer.backup_failed", &[("error", &error)]))?;
                }
                std::fs::write(&self.path, merged)
                    .map_err(|error| tr_args("transfer.cant_write", &[("error", &error)]))?;
                Ok(tr_args("transfer.imported", &[("file", &BEFORE_IMPORT)]))
            });
        self.reload();
        self.notice = Some((SLOT, result.unwrap_or_else(|error| error)));
    }
}
