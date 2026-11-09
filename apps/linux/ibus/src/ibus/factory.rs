//! `org.freedesktop.IBus.Factory`：ibus-daemon 每开一个输入上下文就叫它造一个引擎对象。

use std::sync::atomic::{AtomicU64, Ordering};

use zbus::object_server::ObjectServer;
use zbus::zvariant::OwnedObjectPath;

use super::engine::IBusEngine;
use crate::client::Shared;

/// 引擎对象挂在这个路径下，后面接一个自增序号。
const ENGINE_PATH_PREFIX: &str = "/org/freedesktop/IBus/Engine/Qingjian";

/// 工厂对象。
pub struct IBusFactory {
    /// 与 Server 的共用连接，造出来的引擎对象都用它。
    ///
    /// 一个进程服务多个输入上下文，每个上下文在这条连接上开一个 Server 会话
    /// （组句状态在 Server 那边按会话隔离，这与 Windows Server 的多会话是同一个形状）。
    client: Shared,

    /// 引擎对象路径的序号。
    next: AtomicU64,
}

impl IBusFactory {
    pub fn new(client: Shared) -> Self {
        Self {
            client,
            next: AtomicU64::new(1),
        }
    }
}

#[zbus::interface(name = "org.freedesktop.IBus.Factory")]
impl IBusFactory {
    /// 造一个引擎对象，把它的路径回给 ibus-daemon。
    async fn create_engine(
        &self,
        name: String,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        let serial = self.next.fetch_add(1, Ordering::Relaxed);
        let path = format!("{ENGINE_PATH_PREFIX}/{serial}");
        tracing::info!(engine = %name, %path, "造一个引擎对象");
        let object = OwnedObjectPath::try_from(path.clone())
            .map_err(|error| zbus::fdo::Error::Failed(format!("引擎对象路径不合法：{error}")))?;
        server
            .at(&object, IBusEngine::new(self.client.clone(), path.clone()))
            .await
            .map_err(|error| zbus::fdo::Error::Failed(format!("挂引擎对象失败：{error}")))?;
        Ok(object)
    }
}
