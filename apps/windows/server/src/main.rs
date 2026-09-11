//! 青简 Windows 输入法的 Server 进程入口：读配置、装配 Engine、在命名管道上服务 TSF DLL。
//! 逻辑都在库部分（`qingjian_windows_server`），这里只做装配与启动。
//!
//! release 构建编成 GUI 子系统（无控制台窗口），登录自启时在后台静默跑；日志走文件（见 `init_logging`）。
//! debug 构建保留控制台，方便 `cargo run` 时看 stderr。
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};

use qingjian_core::{Engine, Language};
use qingjian_platform::{Config, LogLevel, resources};
use qingjian_predict::{CloudGlossFiller, CloudPredictor, PredictConfig};
use qingjian_windows_server::{
    AssemblySpec, LanguageModelFiles, Router, RouterConfig, ServerError, assembly,
};

/// 用户数据目录 `%APPDATA%\Qingjian`（配置、密钥、个人释义表都在这里）。非 Windows（本机开发）拿不到。
fn user_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|dir| PathBuf::from(dir).join("Qingjian"))
}

fn config_path() -> Option<PathBuf> {
    user_dir().map(|dir| dir.join("config.toml"))
}

/// 文件不存在按默认值；解析失败不崩，记一条错误退回默认。
fn load_config() -> Config {
    match config_path() {
        Some(path) => Config::load(&path).unwrap_or_else(|error| {
            tracing::error!(%error, path = %path.display(), "配置解析失败，用默认值");
            Config::default()
        }),
        None => Config::default(),
    }
}

/// 读密钥（`QINGJIAN_API_KEY` 等）：工作目录的 `.env`，再叠加 `%APPDATA%\Qingjian\.env`。不覆盖已有环境变量。
fn load_env() {
    let _ = dotenvy::dotenv();
    if let Some(env_file) = user_dir().map(|dir| dir.join(".env")) {
        let _ = dotenvy::from_path(&env_file);
    }
}

/// 学习语言（`[general] learning_language`）；写得不认识按英文。
fn learning_language(config: &Config) -> Language {
    let code = &config.general.learning_language;
    code.parse().unwrap_or_else(|_| {
        tracing::warn!(code, "不认识的学习语言，按英文");
        Language::English
    })
}

/// 按 `[predict]` 接云联想与释义兜底（随包释义表没有的词上屏后问云端，写进个人释义表）。
/// 未开启 / 缺密钥都不致命，退回纯本地候选。
fn attach_cloud(engine: &mut Engine, config: &PredictConfig) {
    if !config.enabled {
        tracing::info!("云联想未开启（[predict] enabled = false）");
        return;
    }
    match CloudPredictor::new(config) {
        Ok(predictor) => {
            engine.set_predictor(Box::new(predictor));
            tracing::info!(model = %config.model, "云联想已接入");
        }
        Err(error) => {
            tracing::warn!(%error, "云联想接入失败（缺 API key？），退回本地候选");
        }
    }
    match CloudGlossFiller::new(config) {
        Ok(filler) => engine.set_gloss_filler(Box::new(filler)),
        Err(error) => tracing::warn!(%error, "释义兜底未启用"),
    }
}

/// 随包生成的数据文件（`<root>/data/generated/<name>`），不存在为 `None`。
fn generated(root: &Path, name: &str) -> Option<PathBuf> {
    existing(root.join("data/generated").join(name))
}

/// 随 git 的资源（`<root>/assets/<rel>`），不存在为 `None`。
fn asset(root: &Path, rel: &str) -> Option<PathBuf> {
    existing(root.join("assets").join(rel))
}

fn existing(path: PathBuf) -> Option<PathBuf> {
    path.is_file().then_some(path)
}

/// 缺省词库：正式词库，没有就回落手写样例。
fn default_dict(root: &Path) -> PathBuf {
    generated(root, "dict.qj").unwrap_or_else(|| sample_dict(root))
}

fn sample_dict(root: &Path) -> PathBuf {
    root.join("assets/sample/dict.tsv")
}

/// 某语言的释义表：打包过的优先，否则随 git 的 TSV；没有为 `None`。
fn glossary_file(root: &Path, language: Language) -> Option<PathBuf> {
    let code = language.code();
    generated(root, &format!("glossary-{code}.qj"))
        .or_else(|| asset(root, &format!("glossary/glossary-{code}.tsv")))
}

/// 正式词库装配失败（如 `.qj` 格式不匹配）回落样例词库，连样例都装不起来才报错。
fn assemble_with_fallback(mut spec: AssemblySpec, root: &Path) -> Result<Engine, ServerError> {
    assembly::assemble(&spec).or_else(|error| {
        tracing::error!(%error, dict = %spec.dict.display(), "正式词库装配失败，回落样例词库");
        spec.dict = sample_dict(root);
        assembly::assemble(&spec)
    })
}

/// 日志目录 `%APPDATA%\Qingjian\logs`（建好返回），拿不到就 `None`（只写 stderr）。
fn log_dir() -> Option<PathBuf> {
    let dir = user_dir()?.join("logs");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// 初始化日志：级别按配置 `[general] log_level`（`RUST_LOG` 可覆盖），同时写 stderr 与按天滚动的日志文件（留 7 天）。
/// 返回非阻塞写入的 guard，要在 `main` 里活到进程结束，否则缓冲的日志不落盘。
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

    let config = load_config();
    // 日志级别取自配置，所以先读配置再装日志（配置解析出错在装好日志前发生，那条错误会丢，罕见可接受）。
    let _log_guard = init_logging(&config);
    let language = learning_language(&config);
    // 随包资源根：装机布局与 exe 同级，开发布局是仓库 `ime/`；都找不到回落工作目录（保留旧的 cwd 相对行为）。
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
    // 与 macOS 壳的 apply_config 对齐：模糊音、双拼、云联想 + 释义兜底。
    engine.set_fuzzy(config.fuzzy);
    engine.set_shuangpin(config.general.shuangpin());
    attach_cloud(&mut engine, &config.predict);
    let router_config = RouterConfig::from(&config);
    let mut router = Router::new(engine, router_config.clone());
    // 配置热加载：改了 config.toml 不用重启 Server（与 macOS 每秒看 mtime 对齐）。
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
        sessions = router.session_count(),
        "青简 Windows Server 就绪"
    );

    serve(router);
}

/// 在命名管道上服务到进程结束。先起候选窗口自绘线程，把它作为 Router 的候选输出端。
// TODO(windows)：本地整句模型的异步结果；焦点离开时把 preedit 上屏。
#[cfg(windows)]
fn serve(mut router: Router) {
    use qingjian_windows_server::ipc::pipe;
    use qingjian_windows_server::ui::CandidateUi;
    // 候选窗口搬到 Server 进程自绘：起 UI 线程作为 Router 的候选输出端。失败不致命，退化为不画候选窗口。
    match CandidateUi::spawn() {
        Ok(ui) => router.set_candidate_sink(Box::new(ui)),
        Err(error) => tracing::error!(%error, "候选窗口 UI 线程启动失败，将不显示候选框"),
    }
    if let Err(error) = pipe::serve_pipe(pipe::DEFAULT_PIPE_NAME, &mut router) {
        tracing::error!(%error, "命名管道服务退出");
        std::process::exit(1);
    }
}

/// 命名管道传输仅 Windows 提供；本机开发只验证装配。
#[cfg(not(windows))]
fn serve(_router: Router) {
    tracing::warn!("命名管道传输仅 Windows 提供；本平台只装配 Engine 供测试");
}
