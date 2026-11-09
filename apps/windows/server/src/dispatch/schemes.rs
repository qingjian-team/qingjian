//! 输入方案的装载：拼音侧（内置或用户双拼）与形码侧按配置找出来挂到引擎上。
//!
//! 随包方案在安装目录的 `data/generated/schemes/`（词库仓库发的 `<id>.qj`），用户方案在 `%APPDATA%\Qingjian\schemes\<id>\`，
//! 找法与「没变不重读」见 [`qingjian_platform::SchemeSlot`]。

use std::path::{Path, PathBuf};

use qingjian_platform::Config;

use super::Router;

/// 随包的方案目录：dicts-fetch 把词库仓库的码表解到这里，安装目录与开发布局是同一个相对路径。
const BUNDLED_SCHEMES: &str = "data/generated/schemes";

/// 随包的方案目录；不在为 `None`。
pub fn find_bundled_schemes(bundled_root: &Path) -> Option<PathBuf> {
    let dir = bundled_root.join(BUNDLED_SCHEMES);
    dir.is_dir().then_some(dir)
}

impl Router {
    /// 启动时把随包方案目录交给路由器，并按当前配置装配一次。
    pub fn configure_schemes(&mut self, config: &Config, bundled_schemes: Option<PathBuf>) {
        self.bundled_schemes = bundled_schemes;
        self.reload_schemes(config);
    }

    /// 主题与启动日志显示的方案名（单开全拼也写「全拼」）。
    pub fn scheme_name(&self) -> String {
        self.scheme_slot.scheme_name(self.config.scheme)
    }

    /// 热加载：按新配置的两条轴重新装配；方案与码表都没变时不重读码表。
    pub(super) fn reload_schemes(&mut self, config: &Config) {
        let custom = qingjian_platform::dirs::user_dirs().map(|dirs| dirs.custom());
        self.scheme_slot.apply(
            &mut self.engine,
            config,
            custom.as_deref(),
            self.bundled_schemes.as_deref(),
        );
    }
}
