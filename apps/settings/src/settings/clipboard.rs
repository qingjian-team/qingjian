//! 实验功能页的剪贴板：选程序加进不记录的名单。

use super::Settings;
use crate::i18n::tr;

impl Settings {
    pub(super) fn exclude_clipboard_app(&mut self) {
        let Some(program) = rfd::FileDialog::new()
            .set_title(tr("experimental.clipboard_pick_app"))
            .add_filter(tr("experimental.clipboard_programs"), &["exe"])
            .pick_file()
        else {
            return;
        };
        let Some(name) = program
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
        else {
            return;
        };
        let mut apps = self.config.clipboard.excluded_apps.clone();
        if !apps.iter().any(|app| app.eq_ignore_ascii_case(&name)) {
            apps.push(name);
            self.write_list("clipboard", "excluded_apps", &apps);
        }
    }
}
