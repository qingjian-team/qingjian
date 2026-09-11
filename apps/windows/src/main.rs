//! 青简 Windows 输入法的 **Server 进程**入口。
//!
//! Windows 的 TSF DLL（`ITfTextInputProcessor`）会被加载进**每一个**应用进程，核心逻辑不能放在
//! DLL 里。所以照 Weasel（WeaselServer）/ 水杉的结构：Core（`qingjian_core::Engine`）跑在这个
//! 独立的 Server 进程，DLL 只把系统按键翻成协议消息发过来、把响应画出去。逻辑在库部分
//! （`qingjian_windows`），这里只做装配与启动。
//!
//! 现状：`Router` 已把协议接到 Engine，跑通「拼音 → 候选 → 选词上屏」的进程内闭环（集成测试在
//! `tests/`）。传输层（命名管道）、TSF DLL、翻页 / 英文模式 / 云联想待接。见 `README.md`。

use std::path::PathBuf;

use qingjian_core::Language;
use qingjian_windows::{Router, assembly};

/// 每页候选数缺省值（接配置后由 `[general] page_size` 决定）。
const DEFAULT_PAGE_SIZE: usize = 9;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // TODO(windows)：词库 / 释义表路径应从 %APPDATA%\Qingjian 与随包 Resources 定位（对应 macOS 的 paths.rs）。
    //   现在从环境变量或仓库内样例读，够把 Server 跑起来。
    let dict = std::env::var_os("QINGJIAN_DICT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("assets/sample/dict.tsv"));
    let glossary = std::env::var_os("QINGJIAN_GLOSSARY")
        .map(PathBuf::from)
        .filter(|path| path.is_file());

    let glossary_ref = glossary.as_deref().map(|path| (Language::English, path));
    let engine = match assembly::assemble(&dict, glossary_ref) {
        Ok(engine) => engine,
        Err(error) => {
            tracing::error!(%error, dict = %dict.display(), "Engine 装配失败");
            std::process::exit(1);
        }
    };
    let mut router = Router::new(engine, DEFAULT_PAGE_SIZE);
    tracing::info!(
        dict = %dict.display(),
        sessions = router.session_count(),
        "青简 Windows Server 就绪"
    );

    // TODO(windows, 下一阶段)：TSF DLL；云联想 / 本地整句模型的异步结果经 ServerMessage::Update 推给会话。
    #[cfg(windows)]
    {
        use qingjian_windows::ipc::pipe;
        if let Err(error) = pipe::serve_pipe(pipe::DEFAULT_PIPE_NAME, &mut router) {
            tracing::error!(%error, "命名管道服务退出");
            std::process::exit(1);
        }
    }
    #[cfg(not(windows))]
    {
        tracing::warn!("命名管道传输仅 Windows 提供；本平台只装配 Engine 供测试");
        let _ = &mut router;
    }
}
