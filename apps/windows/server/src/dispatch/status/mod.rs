//! 全局中英模式与悬浮状态条：模式只有 Server 这一份，DLL 切了用 `ModeChanged` 报来，激活 / 获焦 / 轮询时用
//! `SyncMode` 取走（取的同时说明青简是当前输入法，状态条显示）；切成别的输入法时 DLL 发 `ImeSwitched` 收起。
//! 会话关闭（应用退出）不收——状态条常驻桌面。状态条上的点击经 [`StatusEvent`] 回到这里：
//! 切模式直接改全局模式，各 DLL 下一拍取走；切标点 / 拖动写回配置文件（热加载会再读回来）。

mod event;
mod sink;
mod view;

use qingjian_platform::protocol::IndicatorCommand;
use qingjian_platform::{Config, Scheme, scheme_label};

pub use self::event::StatusEvent;
pub use self::sink::{NoopStatusSink, StatusSink};
pub use self::view::StatusView;
use super::Router;

impl Router {
    /// DLL 那边用户切了模式：成为全局模式。内置英文模式关着时不收英文。
    pub(super) fn handle_mode_changed(&mut self, english: bool) {
        self.english = english && self.config.english_mode;
        self.ime_active = true;
        self.reconcile_status();
    }

    /// 有 DLL 来取模式：青简是当前输入法。
    pub(super) fn handle_ime_active(&mut self) {
        if !self.config.english_mode {
            self.english = false;
        }
        if !self.ime_active {
            self.ime_active = true;
            self.reconcile_status();
        }
    }

    pub(super) fn handle_ime_switched(&mut self) {
        self.ime_active = false;
        self.reconcile_status();
    }

    /// 状态条上的操作。
    pub fn handle_status_event(&mut self, event: StatusEvent) {
        match event {
            StatusEvent::ToggleMode => {
                // 关掉内置英文模式后这一格不切模式：DLL 那边也会拦（配置改了没切走再切回时两边都挡住）
                if !self.config.english_mode {
                    tracing::debug!("内置英文模式已关闭，状态条不切模式");
                    return;
                }
                self.english = !self.english;
                tracing::debug!(english = self.english, "状态条：切换中英模式");
            }
            StatusEvent::TogglePunctuation => {
                // 中英各记一份，切的是当前模式那份；还没报过模式时按中文算。
                let english = self.english;
                let full_width = !self.full_width_for(english);
                let key = if english {
                    self.config.english_full_width = full_width;
                    "english_full_width_punctuation"
                } else {
                    self.config.full_width = full_width;
                    "full_width_punctuation"
                };
                tracing::debug!(english, full_width, "状态条：切换全角标点");
                self.persist("general", key, full_width);
            }
            StatusEvent::Moved(x, y) => {
                self.config.status_pos = Some((x, y));
                self.persist("status_bar", "x", i64::from(x));
                self.persist("status_bar", "y", i64::from(y));
            }
        }
        self.reconcile_status();
    }

    /// 任务栏图标右键菜单：标点与悬浮条的开关写回配置文件（热加载会再读回来），设置程序交给 UI 起。
    pub(super) fn handle_indicator(&mut self, command: IndicatorCommand) {
        match command {
            IndicatorCommand::TogglePunctuation => {
                self.handle_status_event(StatusEvent::TogglePunctuation);
            }
            IndicatorCommand::ToggleStatusBar => {
                self.config.status_enabled = !self.config.status_enabled;
                self.persist("status_bar", "enabled", self.config.status_enabled);
                self.reconcile_status();
            }
            IndicatorCommand::OpenSettings => self.status.open_settings(),
            IndicatorCommand::OpenDownload => self.status.open_download(),
        }
    }

    /// 写回配置文件一个键；没有配置路径（测试）就只改内存。
    fn persist(&self, section: &str, key: &str, value: impl Into<toml_edit::Value>) {
        let Some(path) = self.config_path() else {
            return;
        };
        if let Err(error) = Config::set_value(path, section, key, value) {
            tracing::warn!(%error, section, key, "写回配置失败");
        }
    }

    /// 当前模式下标点转不转全角：中英各一份配置。
    pub(super) fn full_width_for(&self, english: bool) -> bool {
        if english {
            self.config.english_full_width
        } else {
            self.config.full_width
        }
    }

    /// 开着且青简在前台就显示，否则收起。热加载后也调一次。
    pub(super) fn reconcile_status(&mut self) {
        match self.ime_active.then_some(self.english) {
            Some(english) if self.config.status_enabled => {
                self.status.show_status(StatusView {
                    english,
                    zhuyin: self.config.scheme == Scheme::Zhuyin,
                    // 现算，不存下来：存了会与 scheme / wubi 冗余、手搓配置的地方就漂移
                    scheme: Some(scheme_label(self.config.scheme, self.config.wubi))
                        .filter(|label| !label.is_empty()),
                    full_width: self.full_width_for(english),
                    theme: self.config.theme,
                    anchor: self.config.status_pos,
                });
            }
            _ => self.status.hide_status(),
        }
    }
}
