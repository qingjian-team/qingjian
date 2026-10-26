//! 任务栏「中 / 英」图标右键菜单（见 [`crate::com::mode::menu`]）落到文本服务上。

use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    ASFW_ANY, AllowSetForegroundWindow, GetForegroundWindow, GetWindowThreadProcessId,
};

use qingjian_platform::protocol::IndicatorCommand;

use super::TextService_Impl;
use crate::com::log::log;
use crate::com::mode::menu::{self, MenuChoice, MenuState};

impl TextService_Impl {
    pub(super) fn show_indicator_menu(&self, point: POINT) {
        let Some(owner) = self.menu_owner() else {
            log("右键菜单：找不到本线程的窗口，不弹");
            return;
        };
        // 先跟上全局模式，菜单上的勾与悬浮状态条一致（悬浮条上刚点过、还没到轮询那一拍时）
        self.sync_mode_from_server();
        let english = self.mode_state.english();
        let indicator = self.indicator_state.get();
        let state = MenuState {
            english,
            english_enabled: self.mode_state.enabled(),
            full_width: if english {
                indicator.english_full_width_punctuation
            } else {
                indicator.full_width_punctuation
            },
            status_bar: indicator.status_bar,
            update_available: indicator.update_available,
        };
        match menu::track(owner, point, &state) {
            Some(MenuChoice::Mode { english }) if english != self.mode_state.english() => {
                self.set_english_mode(english);
            }
            Some(MenuChoice::Server(IndicatorCommand::OpenSymbolPanel)) => {
                self.send_indicator(IndicatorCommand::OpenSymbolPanel);
                // 当场取一拍状态：面板开了就接着把键交给 Server，不等轮询
                self.sync_mode_from_server();
            }
            Some(MenuChoice::Server(command)) => self.send_indicator(command),
            _ => {}
        }
    }

    fn send_indicator(&self, command: IndicatorCommand) {
        if matches!(
            command,
            IndicatorCommand::OpenSettings
                | IndicatorCommand::OpenLegacySettings
                | IndicatorCommand::OpenDownload
        ) {
            // 设置程序 / 浏览器由 Server 起；前台权在点菜单的这边，让出去它的窗口才能到前面
            let _ = unsafe { AllowSetForegroundWindow(ASFW_ANY) };
        }
        match self.engine.borrow_mut().as_mut() {
            Some(client) => {
                if let Err(error) = client.indicator(command) {
                    log(&format!("右键菜单发给 Server 失败: {error}"));
                }
            }
            None => log("右键菜单：没连上 Server"),
        }
    }

    /// 菜单要挂在本线程的窗口上：先取焦点输入框所在窗口，没有再看前台窗口是不是本线程的。
    fn menu_owner(&self) -> Option<HWND> {
        let from_context = self
            .thread_mgr
            .borrow()
            .as_ref()
            .and_then(|thread_mgr| unsafe {
                let view = thread_mgr
                    .GetFocus()
                    .ok()?
                    .GetTop()
                    .ok()?
                    .GetActiveView()
                    .ok()?;
                view.GetWnd().ok()
            });
        from_context
            .or_else(|| {
                let foreground = unsafe { GetForegroundWindow() };
                let thread = unsafe { GetWindowThreadProcessId(foreground, None) };
                (thread == unsafe { GetCurrentThreadId() }).then_some(foreground)
            })
            .filter(|hwnd| !hwnd.is_invalid())
    }
}
