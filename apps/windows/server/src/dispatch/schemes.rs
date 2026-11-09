//! 输入方案的装载：拼音侧（内置或用户双拼）与形码侧按配置找出来挂到引擎上。
//!
//! 随包方案在安装目录的 `assets/schemes/`，用户方案在 `%APPDATA%\Qingjian\schemes\<id>\`，
//! 找法与「没变不重读」见 [`qingjian_platform::SchemeSlot`]。

use std::path::{Path, PathBuf};

use qingjian_platform::Config;

use super::Router;

/// 随包的方案目录。走 `assets/` 而不是 `data/`：码表随 git 跟踪（`data/` 是给生成物的，
/// 装机时从数据包解出来），与 emoji / levels 一样在仓库与安装目录里是同一个相对路径，
/// 所以 `cargo run` 的开发布局也找得到。
const BUNDLED_SCHEMES: &str = "assets/schemes";

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
        let user_dir = qingjian_platform::dirs::user_dir();
        self.scheme_slot.apply(
            &mut self.engine,
            config,
            user_dir.as_deref(),
            self.bundled_schemes.as_deref(),
        );
    }
}
