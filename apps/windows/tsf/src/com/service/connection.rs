//! 连 Server：激活 / 获焦时开会话，失败按 [`RECONNECT_INTERVAL`] 退避重试；转发出错就断开、下一键重连。

use std::time::Instant;

use super::launch;
use super::{RECONNECT_INTERVAL, TextService_Impl};
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
                if launch::launch_server() {
                    // 不等 RECONNECT_INTERVAL：Server 一百多毫秒就监听管道，下一键就该连上
                    self.last_connect_failure.set(None);
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
        // 默认行为仍向重启后的 Server 报模式；配置了初始模式或应用记忆时以 Server 为准。
        if connected && self.shared.foreground() {
            if self
                .input_settings
                .get()
                .is_some_and(|input| input.restore_mode)
            {
                self.sync_mode_from_server();
            } else {
                self.report_mode();
            }
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
