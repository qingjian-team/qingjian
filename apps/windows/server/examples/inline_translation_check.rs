//! 用隔离的真实 Router 检查中文候选右侧译文，不显示独立 EN／JA 候选。
//! 只读已安装配置，子进程隔离短句缓存；输出布尔值、计数和耗时，不输出原文或密钥。

use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use qingjian_core::{CandidateKind, Engine, Language, Translator};
use qingjian_dictionary::Dictionary;
use qingjian_platform::Config;
use qingjian_platform::protocol::{
    ClientMessage, Frame, KeyEvent, KeyModifiers, PROTOCOL_VERSION, ServerMessage, SessionId,
};
use qingjian_predict::CloudPredictor;
use qingjian_windows_server::{Router, RouterConfig};

const INPUT: &str = "wanfanchishenme";
const LOCAL: &str = "完饭吃什么";

#[derive(Default)]
struct Report {
    setup: bool,
    first_page: bool,
    local_first: bool,
    chinese_cloud: bool,
    inline_translation: bool,
    shortcut_count: usize,
    candidate_count: usize,
    elapsed_ms: u128,
}

#[derive(Debug, Clone, Copy)]
struct EnglishTranslator;

impl Translator for EnglishTranslator {
    fn language(&self) -> Language {
        Language::English
    }

    fn translate(&self, _text: &str) -> Option<qingjian_core::Translation> {
        None
    }
}

fn main() {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() == Some(OsStr::new("--isolated")) {
        let report = match (args.next(), args.next()) {
            (Some(config), Some(isolation)) => run(Path::new(&config), Path::new(&isolation)),
            _ => Report::default(),
        };
        print_report(&report);
        return;
    }
    let status = (|| {
        let config = qingjian_platform::dirs::config_path()?;
        let executable = std::env::current_exe().ok()?;
        let isolation = std::env::current_dir().ok()?.join(format!(
            "inline-translation-no-cache-{}",
            std::process::id()
        ));
        if isolation.exists() {
            return None;
        }
        Command::new(executable)
            .arg("--isolated")
            .arg(config)
            .arg(&isolation)
            .env("APPDATA", &isolation)
            .status()
            .ok()
    })();
    if !status.is_some_and(|status| status.success()) {
        print_report(&Report::default());
    }
}

fn run(config_path: &Path, isolation: &Path) -> Report {
    let mut report = Report::default();
    if !config_path.is_file()
        || isolation.exists()
        || std::env::var_os("APPDATA").as_deref() != Some(isolation.as_os_str())
    {
        return report;
    }
    let Ok(config) = Config::load(config_path) else {
        return report;
    };
    let mut predict = config.predict.clone();
    predict.enabled = true;
    predict.provider = "cloud".to_owned();
    predict.selection_provider = "cloud".to_owned();
    predict.sentence_translation = true;
    predict.sentence = true;
    predict.slots = 2;
    predict.timeout_ms = 15_000;
    predict.debounce_ms = 100;
    let Ok(predictor) = CloudPredictor::new(&predict) else {
        return report;
    };
    let Ok(dictionary) = Dictionary::parse("完饭吃什么\twan fan chi shen me\t9000\n") else {
        return report;
    };
    let engine = Engine::new(dictionary)
        .with_translator(Box::new(EnglishTranslator))
        .with_predictor(Box::new(predictor));
    let mut router_config = RouterConfig::from(&config);
    router_config.page_size = 6;
    router_config.cloud_slots = predict.slots;
    router_config.zhuyin = false;
    let mut router = Router::new(engine, router_config);
    let session = SessionId(9_876_543_214);
    let opened = matches!(
        router.handle(ClientMessage::OpenSession {
            session,
            app: Some("inline-translation-check.exe".to_owned()),
            protocol: PROTOCOL_VERSION,
        }),
        Some(ServerMessage::SessionOpened { .. })
    );
    report.setup = opened;
    if !opened {
        return report;
    }
    let start = Instant::now();
    let frame = compose(&mut router, session, Duration::from_secs(20));
    report.elapsed_ms = start.elapsed().as_millis();
    report.first_page = frame.page == 0;
    report.local_first = frame
        .candidates
        .items
        .first()
        .is_some_and(|candidate| candidate.text == LOCAL);
    report.chinese_cloud = frame
        .candidates
        .items
        .iter()
        .any(|candidate| candidate.kind == CandidateKind::Cloud && candidate.text != LOCAL);
    report.inline_translation = frame.candidates.items.iter().any(|candidate| {
        candidate
            .translation
            .as_ref()
            .is_some_and(|translation| !translation.senses().is_empty())
    });
    report.shortcut_count = frame
        .candidates
        .items
        .iter()
        .filter(|candidate| candidate.kind == CandidateKind::Shortcut)
        .count();
    report.candidate_count = frame.candidates.items.len();
    report
}

fn compose(router: &mut Router, session: SessionId, timeout: Duration) -> Frame {
    router.handle(ClientMessage::Key {
        session,
        event: KeyEvent::new(0x1B, None, KeyModifiers::default()),
    });
    router.engine_mut().set_input(&INPUT[..INPUT.len() - 1]);
    let mut frame = match router.handle(ClientMessage::Key {
        session,
        event: KeyEvent::new(u32::from(b'E'), Some('e'), KeyModifiers::default()),
    }) {
        Some(ServerMessage::KeyResult { frame, .. }) => frame,
        _ => Frame::default(),
    };
    let start = Instant::now();
    while !(frame.candidates.items.iter().any(|candidate| {
        candidate
            .translation
            .as_ref()
            .is_some_and(|translation| !translation.senses().is_empty())
    }) && frame.page == 0)
        && start.elapsed() < timeout
    {
        std::thread::sleep(Duration::from_millis(20));
        if let Some(ServerMessage::Update { frame: next, .. }) =
            router.handle(ClientMessage::Poll { session })
        {
            frame = next;
        }
    }
    frame
}

fn print_report(report: &Report) {
    println!(
        "setup={} first_page={} local_first={} chinese_cloud={} inline_translation={} shortcut_count={} candidate_count={} elapsed_ms={}",
        report.setup,
        report.first_page,
        report.local_first,
        report.chinese_cloud,
        report.inline_translation,
        report.shortcut_count,
        report.candidate_count,
        report.elapsed_ms,
    );
}
