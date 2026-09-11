//! 悬浮状态条：中英模式只在 DLL 侧，DLL 用 `ModeChanged` 推来（激活 / 获焦 / 切换时）；
//! 切成别的输入法时 DLL 发 `ImeSwitched` 收起。会话关闭（应用退出）不收——状态条常驻桌面。
//! 状态条上的点击经 [`StatusEvent`] 回到这里：切模式记成 `pending_mode` 等 DLL 用 `SyncMode` 来取，
//! 切标点 / 拖动写回配置文件（热加载会再读回来）。

mod event;
mod sink;
mod view;

use qingjian_platform::Config;

pub use self::event::StatusEvent;
pub use self::sink::{NoopStatusSink, StatusSink};
pub use self::view::StatusView;
use super::Router;

impl Router {
    pub(super) fn handle_mode_changed(&mut self, english: bool) {
        self.status_mode = Some(english);
        self.reconcile_status();
    }

    pub(super) fn handle_ime_switched(&mut self) {
        self.status_mode = None;
        self.reconcile_status();
    }

    /// DLL 来取状态条上点出的目标模式；取走即清。
    pub(super) fn take_pending_mode(&mut self) -> Option<bool> {
        self.pending_mode.take()
    }

    /// 状态条上的操作。
    pub fn handle_status_event(&mut self, event: StatusEvent) {
        match event {
            StatusEvent::ToggleMode => {
                let Some(english) = self.status_mode else {
                    return;
                };
                // 先把状态条翻过来，DLL 取走后回报 ModeChanged 再对一次账。
                self.pending_mode = Some(!english);
                self.status_mode = Some(!english);
                tracing::debug!(english = !english, "状态条：请求切换中英模式");
            }
            StatusEvent::TogglePunctuation => {
                // 中英各记一份，切的是当前模式那份；还没报过模式时按中文算。
                let english = self.status_mode == Some(true);
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
        match self.status_mode {
            Some(english) if self.config.status_enabled => {
                self.status.show_status(StatusView {
                    english,
                    scheme: self
                        .config
                        .shuangpin
                        .map(|scheme| scheme.label().to_owned()),
                    full_width: self.full_width_for(english),
                    theme: self.config.theme,
                    anchor: self.config.status_pos,
                });
            }
            _ => self.status.hide_status(),
        }
    }
}
