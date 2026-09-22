//! Linux 本地输入服务启动入口；云服务在首版保持关闭，本地整句模型按 `[model]` 开关在后台加载。
#[cfg(target_os = "linux")]
mod paths;

#[cfg(target_os = "linux")]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    use qingjian_core::Language;
    use qingjian_linux_server::{
        AssemblySpec, LanguageModelFiles, Router, RouterConfig, assembly, find_model,
    };
    use qingjian_platform::Config;
    use std::path::PathBuf;
    if std::env::args().any(|arg| arg == "--version") {
        println!("qingjian-linux-server {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let config_path = paths::config_path();
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Config::write_template_if_missing(&config_path)?;
    let config = Config::load(&config_path)?;
    std::fs::create_dir_all(paths::log_dir())?;
    let appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("server")
        .filename_suffix("log")
        .max_log_files(7)
        .build(paths::log_dir())?;
    let (writer, _guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_ansi(false)
        .with_writer(writer)
        .init();
    let root = paths::resource_root().ok_or("product resources missing; set QINGJIAN_RESOURCES")?;
    let dictionary = std::env::var_os("QINGJIAN_DICT")
        .map(PathBuf::from)
        .or_else(|| paths::generated(&root, "dict.qj"))
        .unwrap_or_else(|| root.join("assets/sample/dict.tsv"));
    let language = config
        .general
        .learning_language
        .parse()
        .unwrap_or(Language::English);
    let glossary = |lang: Language| {
        paths::generated(&root, &format!("glossary-{}.qj", lang.code()))
            .or_else(|| paths::asset(&root, &format!("glossary/glossary-{}.tsv", lang.code())))
    };
    let user_dir = paths::user_dir();
    let mut spec = AssemblySpec {
        glossary: (!config.general.learning_language_off())
            .then(|| glossary(language).map(|p| (language, p)))
            .flatten(),
        english_glossary: glossary(Language::Chinese),
        english: paths::generated(&root, "english.tsv")
            .or_else(|| paths::asset(&root, "sample/english.tsv")),
        emoji: ["emoji/emoji-zh.tsv", "emoji/emoji-en.tsv"]
            .into_iter()
            .filter_map(|n| paths::asset(&root, n))
            .collect(),
        language_model: LanguageModelFiles::find(&root.join("data/generated")),
        bundled_dicts_dir: Some(root.join("data/generated/dicts")),
        dictionaries: config.dictionaries.clone(),
        levels_dir: Some(root.join("assets/levels")),
        user_dir: Some(user_dir.clone()),
        input_log: config.general.input_log,
        log_dir: Some(paths::log_dir()),
        ..AssemblySpec::new(dictionary)
    };
    let mut engine = match assembly::assemble(&spec) {
        Ok(engine) => engine,
        Err(error) => {
            tracing::warn!(%error, "主词库不可用，降级样例词库");
            spec.dict = root.join("assets/sample/dict.tsv");
            assembly::assemble(&spec)?
        }
    };
    engine.set_fuzzy(config.fuzzy);
    engine.set_shuangpin(config.general.shuangpin());
    engine.set_zhuyin_mode(config.general.is_zhuyin());
    engine.set_shift_letter_compose(config.general.shift_letter.compose());
    engine.set_learning(config.general.learning);
    engine.set_chinese_first(config.general.chinese_first);
    engine.set_mode_keys(config.shortcut.mode);
    engine
        .set_custom_phrases(config.custom_phrases.clone())
        .map_err(std::io::Error::other)?;
    engine.log_session(env!("CARGO_PKG_VERSION"), "linux");
    let mut router = Router::new(engine, RouterConfig::from(&config));
    router.configure_local_model(find_model(Some(&user_dir), &root), &config.model);
    extern "C" fn stop(_: libc::c_int) {
        qingjian_linux_server::ipc::request_shutdown();
    }
    unsafe {
        libc::signal(libc::SIGTERM, stop as *const () as libc::sighandler_t);
        libc::signal(libc::SIGINT, stop as *const () as libc::sighandler_t);
    }
    let socket = qingjian_linux_server::ipc::socket_path();
    tracing::info!(path = %socket.display(), "青简 Linux Server 启动");
    qingjian_linux_server::ipc::serve_socket(socket, &mut router)?;
    Ok(())
}
fn main() {
    #[cfg(target_os = "linux")]
    if let Err(error) = run() {
        eprintln!("青简启动失败：{error}");
        std::process::exit(1);
    }
    #[cfg(not(target_os = "linux"))]
    eprintln!("qingjian-linux-server 仅支持 Linux");
}
