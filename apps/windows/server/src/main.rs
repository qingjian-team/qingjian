//! 青简 Windows 输入法的 Server 进程入口：读配置、装配 Engine、在命名管道上服务 TSF DLL。
//! 逻辑都在库部分（`qingjian_windows_server`），这里只做装配与启动。

use std::path::{Path, PathBuf};

use qingjian_core::{Engine, Language};
use qingjian_platform::Config;
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

// TODO(windows)：数据文件应从随包 Resources 定位（对应 macOS 的 paths.rs）；现在按仓库工作目录找，
//   生成物在 data/generated/（gitignore），随 git 的在 assets/。

/// 仓库内生成的数据文件（`data/generated/<name>`），不存在为 `None`。
fn generated(name: &str) -> Option<PathBuf> {
    existing(PathBuf::from("data/generated").join(name))
}

fn existing(path: PathBuf) -> Option<PathBuf> {
    path.is_file().then_some(path)
}

/// 缺省词库：正式词库，没有就回落手写样例。
fn default_dict() -> PathBuf {
    generated("dict.qj").unwrap_or_else(sample_dict)
}

fn sample_dict() -> PathBuf {
    PathBuf::from("assets/sample/dict.tsv")
}

/// 某语言的释义表：打包过的优先，否则随 git 的 TSV；没有为 `None`。
fn glossary_file(language: Language) -> Option<PathBuf> {
    let code = language.code();
    generated(&format!("glossary-{code}.qj")).or_else(|| {
        existing(PathBuf::from(format!(
            "assets/glossary/glossary-{code}.tsv"
        )))
    })
}

/// 正式词库装配失败（如 `.qj` 格式不匹配）回落样例词库，连样例都装不起来才报错。
fn assemble_with_fallback(mut spec: AssemblySpec) -> Result<Engine, ServerError> {
    assembly::assemble(&spec).or_else(|error| {
        tracing::error!(%error, dict = %spec.dict.display(), "正式词库装配失败，回落样例词库");
        spec.dict = sample_dict();
        assembly::assemble(&spec)
    })
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    load_env();

    let config = load_config();
    let language = learning_language(&config);
    let dict = std::env::var_os("QINGJIAN_DICT")
        .map(PathBuf::from)
        .unwrap_or_else(default_dict);
    let glossary = std::env::var_os("QINGJIAN_GLOSSARY")
        .map(PathBuf::from)
        .or_else(|| glossary_file(language))
        .filter(|path| path.is_file());
    let spec = AssemblySpec {
        glossary: glossary.clone().map(|path| (language, path)),
        english_glossary: glossary_file(Language::Chinese),
        english: generated("english.tsv"),
        emoji: ["emoji-zh.tsv", "emoji-en.tsv"]
            .into_iter()
            .filter_map(|name| existing(PathBuf::from("assets/emoji").join(name)))
            .collect(),
        language_model: LanguageModelFiles::find(Path::new("data/generated")),
        bundled_dicts_dir: Some(PathBuf::from("data/generated/dicts")).filter(|dir| dir.is_dir()),
        dictionaries: config.dictionaries.clone(),
        levels_dir: Some(PathBuf::from("assets/levels")),
        user_dir: user_dir(),
        input_log: config.general.input_log,
        ..AssemblySpec::new(&dict)
    };
    let mut engine = match assemble_with_fallback(spec) {
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
    let router = Router::new(engine, router_config.clone());
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

/// 在命名管道上服务到进程结束。
// TODO(windows)：本地整句模型的异步结果；焦点离开时把 preedit 上屏。
#[cfg(windows)]
fn serve(mut router: Router) {
    use qingjian_windows_server::ipc::pipe;
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
