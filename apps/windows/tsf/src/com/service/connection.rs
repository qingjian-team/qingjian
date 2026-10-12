//! 连 Server：激活 / 获焦时开会话，失败按 [`RECONNECT_INTERVAL`] 退避重试；转发出错就断开、下一键重连。

use std::time::Instant;

use super::launch;
use super::{LAUNCH_GRACE, RECONNECT_INTERVAL, TextService_Impl};
use crate::client::EngineClient;
use crate::client::pipe::connect_default;
use crate::com::log::log;

impl TextService_Impl {
    /// 连 Server 并开会话（会话号见 [`crate::com::session_id`]，带上宿主 exe 名）。
    pub(super) fn connect(&self) {
        let session = crate::com::session_id();
        let app = crate::com::host_app_name();
        let connected = connect_default()
            .map_err(|e| e.to_string())
            .and_then(|stream| EngineClient::open(stream, session, app).map_err(|e| e.to_string()));
        match connected {
            Ok((client, input)) => {
                *self.engine.borrow_mut() = Some(client);
                self.last_connect_failure.set(None);
                self.launch_grace_until.set(None);
                self.launch_grace_logged.set(false);
                log("已连上 Server");
                // 按键行为设置随 `OpenSession` 的回包一起下来（DLL 不读配置文件，AppContainer 里读不到）。
                self.apply_input_settings(input);
            }
            Err(error) => {
                self.last_connect_failure.set(Some(Instant::now()));
                log(&format!(
                    "连 Server 失败（qingjian-server 没起？）: {error}"
                ));
                // Server 只在登录时由「启动」文件夹拉起，中途挂了以前只能等下次登录；
                // 这里自己起一次（进程内冷却 + 跨进程互斥体，不会砸出一串 Server）。
                match launch::launch_server() {
                    launch::LaunchOutcome::Requested | launch::LaunchOutcome::InFlight => {
                        // 不等 RECONNECT_INTERVAL：Server 一百多毫秒就监听管道，下一键就该连上。
                        self.last_connect_failure.set(None);
                        // 拉起在路上：给一小段宽限继续吞拼音键。只在这次断连里记一次（`is_none` 挡住），
                        // 别让每 5 秒一次的重试把宽限无限续下去——宽限到点必须真的放行。
                        if self.launch_grace_until.get().is_none() {
                            self.launch_grace_until
                                .set(Some(Instant::now() + LAUNCH_GRACE));
                            self.launch_grace_logged.set(false);
                        }
                    }
                    launch::LaunchOutcome::Refused => {
                        // 这个宿主里起不出 Server（登录界面 / 商店应用 / 找不到 exe）：
                        // 短期不会有 Server，宽限立即过期，字母直接放行——死键盘比漏字母难看。
                        self.launch_grace_until.set(Some(Instant::now()));
                    }
                }
            }
        }
    }

    /// 没连上就重连一次（距上次失败不到 [`RECONNECT_INTERVAL`] 则跳过）。返回此刻是否连着。
    pub(super) fn ensure_connected(&self) -> bool {
        if self.engine.borrow().is_some() {
            return true;
        }
        let recently_failed = self
            .last_connect_failure
            .get()
            .is_some_and(|at| at.elapsed() < RECONNECT_INTERVAL);
        if recently_failed {
            return false;
        }
        self.connect();
        let connected = self.engine.borrow().is_some();
        // 重连上的多半是重启过的 Server，它不知道当前模式：前台这边报一次，成为全局模式。
        if connected && self.shared.foreground() {
            self.report_mode();
        }
        connected
    }

    /// 转发失败后断开，下一键重连。
    pub(super) fn disconnect(&self) {
        *self.engine.borrow_mut() = None;
        self.last_connect_failure.set(None);
        self.shared.end_composing();
    }
}
