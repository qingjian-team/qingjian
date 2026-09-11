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
use qingjian_platform::Config;
use qingjian_windows::{Router, assembly};

/// 用户配置文件：`%APPDATA%\Qingjian\config.toml`（对应 macOS 的 `~/Library/Application Support/Qingjian`）。
/// 非 Windows（本机开发）拿不到 `APPDATA`，返回 `None` → 用默认配置。
fn config_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|dir| PathBuf::from(dir).join("Qingjian").join("config.toml"))
}

/// 读配置：文件不存在按默认值（[`Config::load`] 已处理）；解析失败不崩，记一条错误退回默认。
fn load_config() -> Config {
    match config_path() {
        Some(path) => Config::load(&path).unwrap_or_else(|error| {
            tracing::error!(%error, path = %path.display(), "配置解析失败，用默认值");
            Config::default()
        }),
        None => Config::default(),
    }
}

/// 缺省词库：优先仓库内生成的正式词库（`data/generated/dict.qj`，8.7 万词），没有就回落手写样例。
/// 正式词库是 gitignore 的生成物，`cargo run --release -p qingjian-dict-convert -- pack dict …` 产出。
fn default_dict() -> PathBuf {
    let generated = PathBuf::from("data/generated/dict.qj");
    if generated.is_file() {
        generated
    } else {
        PathBuf::from("assets/sample/dict.tsv")
    }
}

/// 缺省释义表：仓库内的英文释义（`assets/glossary/glossary-en.tsv`，随 git 走）；没有就不带释义。
fn default_glossary() -> Option<PathBuf> {
    let glossary = PathBuf::from("assets/glossary/glossary-en.tsv");
    glossary.is_file().then_some(glossary)
}

/// 装配 Engine：正式词库万一加载失败（如 `.qj` 格式不匹配），回落到样例词库，别让 `cargo run` 直接崩。
/// 连样例都装不起来才退出。
fn assemble_with_fallback(
    dict: &std::path::Path,
    glossary: Option<&std::path::Path>,
) -> qingjian_core::Engine {
    let glossary_ref = glossary.map(|path| (Language::English, path));
    match assembly::assemble(dict, glossary_ref) {
        Ok(engine) => engine,
        Err(error) => {
            let sample = PathBuf::from("assets/sample/dict.tsv");
            tracing::error!(%error, dict = %dict.display(), "正式词库装配失败，回落样例词库");
            match assembly::assemble(&sample, glossary_ref) {
                Ok(engine) => engine,
                Err(error) => {
                    tracing::error!(%error, "样例词库也装配失败");
                    std::process::exit(1);
                }
            }
        }
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // TODO(windows)：词库 / 释义表路径应从 %APPDATA%\Qingjian 与随包 Resources 定位（对应 macOS 的 paths.rs）。
    //   现在从环境变量或仓库内的正式数据读（缺省即正式词库 + 英文释义，方便本机 `cargo run` 直接试真数据）。
    let dict = std::env::var_os("QINGJIAN_DICT")
        .map(PathBuf::from)
        .unwrap_or_else(default_dict);
    let glossary = std::env::var_os("QINGJIAN_GLOSSARY")
        .map(PathBuf::from)
        .or_else(default_glossary)
        .filter(|path| path.is_file());

    let config = load_config();
    let general = &config.general;
    let engine = assemble_with_fallback(&dict, glossary.as_deref());
    let mut router = Router::new(engine, general.page_size(), general.layout, general.theme);
    tracing::info!(
        dict = %dict.display(),
        glossary = glossary.as_deref().map(|p| p.display().to_string()).unwrap_or_default(),
        page_size = general.page_size(),
        layout = general.layout.key(),
        theme = general.theme.key(),
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
