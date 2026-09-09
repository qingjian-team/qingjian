//! 青简 Windows 输入法的 **Server 进程**（骨架）。
//!
//! Windows 的 TSF DLL（`ITfTextInputProcessor`）会被加载进**每一个**应用进程，核心逻辑不能放在
//! DLL 里。所以照 Weasel（WeaselServer）/ 水杉的结构：Core（[`qingjian_core::Engine`]）跑在这个
//! 独立的 Server 进程，DLL 只把系统按键翻成 [`qingjian_platform::protocol::ClientMessage`] 发过来、
//! 把 [`qingjian_platform::protocol::ServerMessage`] 画出去。协议两端共用，定义在
//! [`qingjian_platform::protocol`]。
//!
//! 目前是骨架：会话的分派（[`dispatch::Router`]）已成形，还没接真正的传输（命名管道）、Engine
//! 装配与 TSF DLL。见 `apps/windows/README.md` 的分阶段计划。**无法 `cargo run` 验证**，
//! 交叉编译到 Windows 机器上编。

mod dispatch;
mod session;

use qingjian_platform::protocol::{ClientMessage, KeyEvent, SessionId};

use dispatch::Router;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("青简 Windows Server 骨架启动；传输层（命名管道）与 Engine 装配待接");
    smoke_check();

    // TODO(windows, 下一阶段)：
    //   1. 建命名管道 `\\.\pipe\qingjian`，每个连上来的 DLL 客户端一条会话；
    //   2. 装配 Engine（词库 / lm / neural CPU / translator / learner），与 macOS 的 host::init 对齐；
    //   3. 循环读 ClientMessage → Router::handle → 写 ServerMessage 回去；
    //   4. 云联想 / 本地整句模型的异步结果经 ServerMessage::Update 主动推给对应会话。
}

/// 启动自检：把一次「开会话 → 按键 → 关会话」在内存里走一遍，确认协议与分派链路通。
/// 只发一条日志，不产生任何对外行为。
fn smoke_check() {
    let mut router = Router::new();
    let session = SessionId(1);
    router.handle(ClientMessage::OpenSession { session });
    let _ = router.handle(ClientMessage::Key {
        session,
        event: KeyEvent::new(b'n' as u32, Some('n'), Default::default()),
    });
    router.handle(ClientMessage::CloseSession { session });
    tracing::debug!(remaining = router.session_count(), "自检完成");
}
