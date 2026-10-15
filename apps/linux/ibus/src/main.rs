//! 青简 IBus 前端。
//!
//! 与 Fcitx5 插件（`apps/linux/fcitx5`）平级的第二个前端：都只做「框架事件 → Server、帧 → 面板」，
//! 引擎在 `qingjian-linux-server` 那个进程里，两个前端可以同时连着同一个 Server
//! （Server 按连接代次 + 上下文隔离会话）。
//!
//! 三个入口：`--ibus` 跑引擎（ibus-daemon 按组件 XML 拉起的就是它）、`--ibus-xml` 打印组件 XML、
//! `--check` 不碰 IBus 直接对 Server 打一串键（装机排错用）。
//!
//! 真机上的坑记在 `docs/notes/linux-bringup.md`。

mod client;
mod error;
mod ibus;
mod key;

use qingjian_platform::protocol::linux::{Capabilities, socket_path};

use client::Shared;
use error::IbusError;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--ibus") => {
            if let Err(error) = serve() {
                tracing::error!(%error, "IBus 引擎没起来");
                std::process::exit(1);
            }
        }
        Some("--ibus-xml") => {
            // 装包时这份 XML 落到 /usr/share/ibus/component/qingjian.xml，
            // 开发时写进 $IBUS_COMPONENT_PATH 指的目录
            let exec = args
                .next()
                .unwrap_or_else(|| "/usr/bin/qingjian-linux-ibus".to_owned());
            print!("{}", ibus::component::xml(&exec));
        }
        Some("--check") => {
            let typed = args.next().unwrap_or_else(|| "nihao".to_owned());
            if let Err(error) = check(&typed) {
                tracing::error!(%error, "自检没通过");
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!(
                "用法：qingjian-linux-ibus <--ibus | --ibus-xml [可执行文件路径] | --check [拼音]>"
            );
            eprintln!("  --ibus      连上 IBus 总线跑引擎（ibus-daemon 拉起的就是这条）");
            eprintln!("  --ibus-xml  打印组件 XML，放到 /usr/share/ibus/component/");
            eprintln!("  --check     不碰 IBus，直接对 Server 打一串键看回来的候选");
            eprintln!("              两条都要 Server 起着：cargo run -p qingjian-linux-server");
            std::process::exit(2);
        }
    }
}

/// 连上 IBus 总线跑引擎，直到进程被收掉。
///
/// **不在这里连 Server**：连接是按输入上下文懒建的（见 `client::Shared`），
/// Server 晚起、中途重启都不该让引擎进程起不来或者退出——ibus-daemon 拉起我们的时机
/// 比用户起 Server 早得多。
fn serve() -> Result<(), zbus::Error> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| zbus::Error::InputOutput(std::sync::Arc::new(error)))?;
    runtime.block_on(async {
        let client = Shared::default();
        let _connection = ibus::service::serve(client).await?;
        tracing::info!("IBus 引擎就绪，等 ibus-daemon 派活");
        tokio::signal::ctrl_c()
            .await
            .map_err(|error| zbus::Error::InputOutput(std::sync::Arc::new(error)))?;
        tracing::info!("收到中断，退出");
        Ok(())
    })
}

/// 自检：连上 Server、开一个会话、把 `typed` 里的字母逐个当按键报上去，打印最后一帧的候选。
///
/// 这条路走通就说明前端这一侧的协议是对的——**IBus 还没接进来的时候，它是唯一能端到端验的东西**。
fn check(typed: &str) -> Result<(), IbusError> {
    let shared = Shared::default();
    tracing::info!(socket = %socket_path().display(), "连 Server");
    // 会话由 Shared 按上下文标识建，开的时候会带上能力（Server 在收到能力之前不收按键）
    let context = "/qingjian/check";
    shared.set_capabilities(context, Capabilities::default())?;
    let frame = shared.with_session(context, |connection, session| {
        // 真前端在输入框拿到焦点时报这一条，Server 据此把这个会话设成当前焦点
        connection.focus(session, true)?;
        let mut last = Default::default();
        for character in typed.chars() {
            let event = key::to_key_event(character as u32, 0);
            let reply = connection.key(session, event, false)?;
            tracing::debug!(key = %character, outcome = ?reply.outcome, "按键");
            if let Some(commit) = &reply.commit {
                tracing::info!(text = %commit, "上屏");
            }
            last = reply.frame;
        }
        connection.focus(session, false)?;
        Ok(last)
    })?;
    shared.close(context);

    tracing::info!(connected = shared.connected(), "自检走完");
    println!("打了「{typed}」，Server 回的一帧：");
    if frame.candidates.items.is_empty() {
        println!("  （没有候选）");
    }
    for (index, candidate) in frame.candidates.items.iter().enumerate() {
        println!("  {}. {}", index + 1, candidate.text);
    }
    Ok(())
}
