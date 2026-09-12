//! Server 进程入口：读配置、装配 Engine、在命名管道上服务 TSF DLL。逻辑在库部分，这里只装配与启动。
//! release 编成 GUI 子系统（登录自启静默跑，日志走文件）；debug 保留控制台看 stderr。
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};

use qingjian_core::{Engine, Language};
use qingjian_platform::{Config, LogLevel, resources};
use qingjian_windows_server::{
    AssemblySpec, LanguageModelFiles, Router, RouterConfig, ServerError, assembly, dispatch,
};

/// 用户数据目录 `%APPDATA%\Qingjian`。非 Windows 拿不到。
fn user_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|dir| PathBuf::from(dir).join("Qingjian"))
}

fn config_path() -> Option<PathBuf> {
    user_dir().map(|dir| dir.join("config.toml"))
}

/// 文件不存在按默认值；解析失败记错误退回默认。
fn load_config() -> Config {
    match config_path() {
        Some(path) => Config::load(&path).unwrap_or_else(|error| {
            tracing::error!(%error, path = %path.display(), "配置解析失败，用默认值");
            Config::default()
        }),
        None => Config::default(),
    }
}

/// 读密钥：工作目录 `.env`，再叠加 `%APPDATA%\Qingjian\.env`；不覆盖已有环境变量。
fn load_env() {
    let _ = dotenvy::dotenv();
    if let Some(env_file) = user_dir().map(|dir| dir.join(".env")) {
        let _ = dotenvy::from_path(&env_file);
    }
}

fn learning_language(config: &Config) -> Language {
    let code = &config.general.learning_language;
    code.parse().unwrap_or_else(|_| {
        tracing::warn!(code, "不认识的学习语言，按英文");
        Language::English
    })
}

/// `<root>/data/generated/<name>`，不存在为 `None`。
fn generated(root: &Path, name: &str) -> Option<PathBuf> {
    existing(root.join("data/generated").join(name))
}

/// `<root>/assets/<rel>`，不存在为 `None`。
fn asset(root: &Path, rel: &str) -> Option<PathBuf> {
    existing(root.join("assets").join(rel))
}

fn existing(path: PathBuf) -> Option<PathBuf> {
    path.is_file().then_some(path)
}

/// 正式词库，没有就回落手写样例。
fn default_dict(root: &Path) -> PathBuf {
    generated(root, "dict.qj").unwrap_or_else(|| sample_dict(root))
}

fn sample_dict(root: &Path) -> PathBuf {
    root.join("assets/sample/dict.tsv")
}

/// 某语言的释义表：打包过的优先，否则随 git 的 TSV。
fn glossary_file(root: &Path, language: Language) -> Option<PathBuf> {
    let code = language.code();
    generated(root, &format!("glossary-{code}.qj"))
        .or_else(|| asset(root, &format!("glossary/glossary-{code}.tsv")))
}

/// 正式词库装配失败回落样例词库，连样例都装不起来才报错。
fn assemble_with_fallback(mut spec: AssemblySpec, root: &Path) -> Result<Engine, ServerError> {
    assembly::assemble(&spec).or_else(|error| {
        tracing::error!(%error, dict = %spec.dict.display(), "正式词库装配失败，回落样例词库");
        spec.dict = sample_dict(root);
        assembly::assemble(&spec)
    })
}

fn log_dir() -> Option<PathBuf> {
    let dir = user_dir()?.join("logs");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// 级别按 `[general] log_level`（`RUST_LOG` 可覆盖），同时写 stderr 与按天滚动的文件（留 7 天）。
/// 返回的 guard 要活到进程结束，否则缓冲的日志不落盘。
fn init_logging(config: &Config) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_subscriber::fmt::writer::MakeWriterExt;
    let level = if config.general.log_level == LogLevel::Debug {
        "debug"
    } else {
        "info"
    };
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));
    match log_dir() {
        Some(dir) => {
            let appender = tracing_appender::rolling::RollingFileAppender::builder()
                .rotation(tracing_appender::rolling::Rotation::DAILY)
                .filename_prefix("qingjian-server")
                .filename_suffix("log")
                .max_log_files(7)
                .build(&dir)
                .expect("构建滚动日志文件");
            let (writer, guard) = tracing_appender::non_blocking(appender);
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_ansi(false)
                .with_writer(writer.and(std::io::stderr))
                .init();
            Some(guard)
        }
        None => {
            tracing_subscriber::fmt().with_env_filter(filter).init();
            None
        }
    }
}

fn main() {
    load_env();

    // 日志级别取自配置，所以先读配置再装日志。
    let config = load_config();
    let _log_guard = init_logging(&config);
    let language = learning_language(&config);
    // 装机布局与 exe 同级，开发布局是仓库 `ime/`；都找不到回落工作目录。
    let root = resources::bundled_root().unwrap_or_else(|| PathBuf::from("."));
    let dict = std::env::var_os("QINGJIAN_DICT")
        .map(PathBuf::from)
        .unwrap_or_else(|| default_dict(&root));
    let glossary = std::env::var_os("QINGJIAN_GLOSSARY")
        .map(PathBuf::from)
        .or_else(|| glossary_file(&root, language))
        .filter(|path| path.is_file());
    let bundled_dicts_dir = Some(root.join("data/generated/dicts")).filter(|dir| dir.is_dir());
    let spec = AssemblySpec {
        glossary: glossary.clone().map(|path| (language, path)),
        english_glossary: glossary_file(&root, Language::Chinese),
        english: generated(&root, "english.tsv"),
        emoji: ["emoji-zh.tsv", "emoji-en.tsv"]
            .into_iter()
            .filter_map(|name| asset(&root, &format!("emoji/{name}")))
            .collect(),
        language_model: LanguageModelFiles::find(&root.join("data/generated")),
        bundled_dicts_dir: bundled_dicts_dir.clone(),
        dictionaries: config.dictionaries.clone(),
        levels_dir: Some(root.join("assets/levels")),
        user_dir: user_dir(),
        input_log: config.general.input_log,
        ..AssemblySpec::new(&dict)
    };
    let mut engine = match assemble_with_fallback(spec, &root) {
        Ok(engine) => engine,
        Err(error) => {
            tracing::error!(%error, "样例词库也装配失败");
            std::process::exit(1);
        }
    };
    engine.set_fuzzy(config.fuzzy);
    engine.set_shuangpin(config.general.shuangpin());
    engine.set_mode_keys(config.shortcut.mode);
    engine.log_session(env!("CARGO_PKG_VERSION"), "windows");
    dispatch::attach_cloud(&mut engine, &config.predict);
    let router_config = RouterConfig::from(&config);
    let mut router = Router::new(engine, router_config.clone());
    let model_path = dispatch::find_model(user_dir().as_deref(), &root);
    router.configure_local_model(model_path.clone(), &config.model);
    if let Some(path) = config_path() {
        router.watch_config(&config, path, bundled_dicts_dir, user_dir());
    }
    tracing::info!(
        dict = %dict.display(),
        glossary = glossary.as_deref().map(|p| p.display().to_string()).unwrap_or_default(),
        language = language.code(),
        page_size = router_config.page_size,
        page_keys = %format!("{}{}", router_config.page_keys.0, router_config.page_keys.1),
        layout = router_config.layout.key(),
        theme = router_config.theme.key(),
        shuangpin = config.general.shuangpin().map(|s| s.key()).unwrap_or("全拼"),
        fuzzy = config.fuzzy.any(),
        cloud = config.predict.enabled,
        model = model_path.as_deref().map(|p| p.display().to_string()).unwrap_or_default(),
        model_enabled = config.model.enabled,
        sessions = router.session_count(),
        "青简 Windows Server 就绪"
    );

    serve(router);
}

/// DLL 日志目录 `%LOCALAPPDATA%\Qingjian` 给 AppContainer 应用（任务栏搜索 / 设置）写权限：
/// 那些进程里的 DLL 默认写不了用户目录，出了问题连日志都没有。失败只记警告。
#[cfg(windows)]
fn grant_appcontainer_log_access() {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let Some(dir) =
        std::env::var_os("LOCALAPPDATA").map(|base| PathBuf::from(base).join("Qingjian"))
    else {
        return;
    };
    if let Err(error) = std::fs::create_dir_all(&dir) {
        tracing::warn!(%error, dir = %dir.display(), "建 DLL 日志目录失败");
        return;
    }
    // S-1-15-2-1 = ALL APPLICATION PACKAGES，S-1-15-2-2 = ALL RESTRICTED APPLICATION PACKAGES。
    let status = std::process::Command::new("icacls")
        .arg(&dir)
        .args(["/grant", "*S-1-15-2-1:(OI)(CI)M"])
        .args(["/grant", "*S-1-15-2-2:(OI)(CI)M"])
        .creation_flags(CREATE_NO_WINDOW)
        .status();
    match status {
        Ok(status) if status.success() => {}
        Ok(status) => tracing::warn!(%status, "给 AppContainer 授权 DLL 日志目录失败"),
        Err(error) => tracing::warn!(%error, "跑 icacls 失败"),
    }
}

/// 起 UI 线程作为候选窗口 / 状态条的输出端（失败退化为不画），再在命名管道上服务到进程结束。
#[cfg(windows)]
fn serve(mut router: Router) {
    use qingjian_windows_server::ipc::{Work, pipe};
    use qingjian_windows_server::ui::UiHandle;
    grant_appcontainer_log_access();
    // 工人循环的活：各连接的消息 + 状态条上的操作（UI 线程投进来）。
    let (work_tx, work_rx) = std::sync::mpsc::channel::<Work>();
    let status_events = work_tx.clone();
    let on_status = Box::new(move |event| {
        let _ = status_events.send(Work::Status(event));
    });
    match UiHandle::spawn(on_status) {
        Ok(ui) => {
            router.set_candidate_sink(Box::new(ui.clone()));
            router.set_status_sink(Box::new(ui));
        }
        Err(error) => tracing::error!(%error, "UI 线程启动失败，将不显示候选框 / 状态条"),
    }
    if let Err(error) = pipe::serve_pipe(pipe::DEFAULT_PIPE_NAME, &mut router, work_tx, work_rx) {
        tracing::error!(%error, "命名管道服务退出");
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn serve(_router: Router) {
    tracing::warn!("命名管道传输仅 Windows 提供；本平台只装配 Engine 供测试");
}
