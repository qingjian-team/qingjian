//! 本机服务的租约：客户端拿着它服务就开着，最后一份放手时后台线程停掉服务。

use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, PoisonError, Weak};
use std::time::Duration;

use super::error::LocalServerError;
use super::spec::LaunchSpec;
use super::status::{State, Status};
use super::supervisor::Supervisor;

/// 进程里还开着的服务。存弱引用：没有客户端拿着就自然停掉，不靠谁记得去关。
static SERVERS: Mutex<Vec<Weak<LocalServer>>> = Mutex::new(Vec::new());

pub(crate) struct LocalServer {
    /// 启动参数，判断能不能共用。
    spec: LaunchSpec,

    /// 后台线程写的状态。
    status: Arc<Status>,

    /// 不发东西，只为随租约一起放下：后台线程等的就是这个通道断开。
    _stop: Sender<()>,
}

impl LocalServer {
    /// 租用参数为 `spec` 的服务：进程里已有同样参数的就共用；没有就另起一个（参数变了的旧服务等最后一个客户端放手后自己停，
    /// 新的等它让出端口再起）。热加载时壳先建新客户端、再放下旧的，参数没变就不会重启服务。
    pub fn lease(spec: LaunchSpec) -> Arc<Self> {
        let mut servers = SERVERS.lock().unwrap_or_else(PoisonError::into_inner);
        servers.retain(|server| server.strong_count() > 0);
        if let Some(server) = servers
            .iter()
            .filter_map(Weak::upgrade)
            .find(|server| server.spec == spec)
        {
            return server;
        }
        let server = Arc::new(Self::start(spec));
        servers.push(Arc::downgrade(&server));
        server
    }

    fn start(spec: LaunchSpec) -> Self {
        let status = Arc::new(Status::new());
        let (stop, stopped) = mpsc::channel();
        let supervisor = Supervisor::new(spec.clone(), Arc::clone(&status), stopped);
        let spawned = std::thread::Builder::new()
            .name("qingjian-local-model".to_owned())
            .spawn(move || supervisor.run());
        if let Err(error) = spawned {
            status.set(State::Failed(LocalServerError::Spawn(error.to_string())));
        }
        Self {
            spec,
            status,
            _stop: stop,
        }
    }

    /// 现在能不能发请求：`Ok(true)` 能，`Ok(false)` 还在起，`Err` 起不来（后台线程过一阵会再试）。
    pub fn check(&self) -> Result<bool, LocalServerError> {
        match self.status.get() {
            State::Ready => Ok(true),
            State::Starting => Ok(false),
            State::Failed(error) => Err(error),
        }
    }

    /// 等到能用或起不来，最多等 `timeout`；到时还在起报 [`LocalServerError::Loading`]。
    pub fn wait_ready(&self, timeout: Duration) -> Result<(), LocalServerError> {
        match self.status.wait(timeout) {
            State::Ready => Ok(()),
            State::Starting => Err(LocalServerError::Loading),
            State::Failed(error) => Err(error),
        }
    }
}
