//! 辅码码表的装配：`codes/` 里导入的 `.qj` 与 `schemes/` 里的辅码方案，配置或文件变了就重装。

use qingjian_platform::code_tables::AuxCodeSources;

use super::Host;
use crate::app::paths;

impl Host {
    /// 按当前 `[aux_code]` 重装码表，记下这次的配置与文件快照。
    pub fn reload_aux_codes(&mut self) {
        let config = self.settings.config().aux_code.clone();
        let sources = aux_sources();
        let tables = sources.load(&config);
        tracing::info!(count = tables.len(), "辅码码表已装配");
        self.engine.set_aux_codes(tables);
        self.aux_files = sources.snapshot();
        self.applied_aux_code = Some(config);
    }

    /// 设置程序导入了码表、或改了辅码方案（配置不一定变）：文件快照变了就重装。只 stat，不读码表。
    pub(super) fn reload_aux_codes_if_changed(&mut self) {
        if aux_sources().snapshot() != self.aux_files {
            self.reload_aux_codes();
        }
    }
}

fn aux_sources() -> AuxCodeSources {
    AuxCodeSources {
        bundled_codes: paths::bundled_codes_dir(),
        user_codes: paths::codes_dir(),
        bundled_schemes: paths::bundled_schemes_dir(),
        user_dir: paths::user_data_dir(),
    }
}
