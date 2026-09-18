//! 辅码在壳里的闭环：触发与逐键即筛、帧上的码段、筛空与退格边界、标点先上屏、
//! 码表热加载、老 DLL 降级，以及自绘窗与 DLL 面各看各的帧。

use std::sync::{Arc, Mutex};

use qingjian_dictionary::{AuxCodeLookup, AuxCodeTable};
use qingjian_platform::Config;
use qingjian_platform::protocol::{Frame, PreeditKind, ScreenRect};
use qingjian_windows_server::dispatch::{CandidateSink, DataDirs, RenderSettings};

use super::support::*;

/// 装了辅码码表的 Router：`kaifa` 之后敲触发键进辅码态、`k` 筛到 开发。
/// 码表由 Core 的 [`AuxCodeLookup`] 注入；辅码总闸缺省关，这里显式打开。
fn router_with_aux(config: RouterConfig) -> Router {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let dict = root.join("assets/sample/dict.tsv");
    let glossary = root.join("assets/sample/glossary-en.tsv");
    let mut engine = assembly::assemble(&AssemblySpec {
        glossary: Some((Language::English, glossary)),
        english: Some(root.join("assets/sample/english.tsv")),
        ..AssemblySpec::new(dict)
    })
    .expect("assemble engine from sample data");
    let table: Arc<dyn AuxCodeLookup> = Arc::new(
        AuxCodeTable::from_pairs([
            ("开发".to_owned(), "kf".to_owned()),
            ("开发者".to_owned(), "kfz".to_owned()),
            ("开".to_owned(), "kh".to_owned()),
        ])
        .unwrap(),
    );
    engine.set_aux_codes(vec![table]);
    engine.set_aux_enabled(true);
    engine.set_aux_code_key(config.aux_code_key, config.page_keys);
    // 生产接线里 Engine 侧的同名开关与 RouterConfig 同源设置（main.rs），这里对齐
    engine.set_shift_letter_compose(config.shift_letter_compose);
    let mut router = Router::new(engine, config);
    // 协议 6 起开会话回一条按键行为设置（SessionOpened），support 的 helper 负责吃掉它
    open_session(&mut router, SESSION, None);
    router
}

/// 缺省配置 + 码表。
fn aux_router() -> Router {
    router_with_aux(RouterConfig::default())
}

/// 记录自绘候选窗收到的帧。
#[derive(Clone, Default)]
struct RecordingCandidates(Arc<Mutex<Vec<Frame>>>);

impl CandidateSink for RecordingCandidates {
    fn show(&self, frame: Frame, _rect: ScreenRect) {
        self.0.lock().unwrap().push(frame);
    }

    fn hide(&self) {}

    fn configure(&self, _settings: RenderSettings) {}
}

fn rect() -> ScreenRect {
    ScreenRect {
        left: 0,
        top: 0,
        right: 100,
        bottom: 20,
    }
}

/// 辅码态：触发键进状态、码段按 AuxCode 段下发、逐键即筛、筛空留 preedit、退格逐级放宽。
#[test]
fn aux_trigger_filters_candidates_and_marks_the_code_segment() {
    let mut router = aux_router();
    let (_, _, frame) = type_letters(&mut router, "kaifa");
    let before = candidate_texts(&frame).len();
    assert!(before > 1);

    let (outcome, committed, frame) = press(&mut router, letter(';'));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(committed.is_none());
    assert_eq!(preedit(&frame), "kai'fa;");
    assert_eq!(candidate_texts(&frame).len(), before);
    assert!(!frame.preedit.iter().any(|s| s.kind == PreeditKind::AuxCode));

    let (_, _, frame) = press(&mut router, letter('k'));
    assert_eq!(preedit(&frame), "kai'fa;k");
    let segment = frame
        .preedit
        .iter()
        .find(|s| s.kind == PreeditKind::AuxCode)
        .expect("code segment rides the frame");
    assert_eq!(segment.text, "k");
    assert!(candidate_texts(&frame).contains(&"开发"));
    assert!(candidate_texts(&frame).contains(&"开发者"));
    assert!(!candidate_texts(&frame).contains(&"开放"));

    let (_, _, frame) = press(&mut router, letter('f'));
    assert_eq!(candidate_texts(&frame)[0], "开发");
    assert_eq!(frame.candidates.items[0].aux_code.as_deref(), Some("kf"));

    let (_, _, frame) = press(&mut router, letter('q'));
    assert!(frame.candidates.items.is_empty());
    assert!(!frame.is_empty());
    assert_eq!(preedit(&frame), "kai'fa;kfq");

    press(&mut router, function_key(0x08));
    press(&mut router, function_key(0x08));
    let (_, _, frame) = press(&mut router, function_key(0x08));
    assert_eq!(preedit(&frame), "kai'fa;");
    assert_eq!(candidate_texts(&frame).len(), before);
    let (_, _, frame) = press(&mut router, function_key(0x08));
    assert_eq!(preedit(&frame), "kai'fa");
    assert_eq!(candidate_texts(&frame).len(), before);
}

/// `[general] aux_code_key` 换成 `/`：`;` 不再是触发键，`/` 才是。
#[test]
fn aux_trigger_key_follows_config() {
    let mut router = router_with_aux(RouterConfig {
        aux_code_key: '/',
        ..RouterConfig::default()
    });
    type_letters(&mut router, "kaifa");
    let (_, _, frame) = press(&mut router, letter(';'));
    assert!(frame.preedit.iter().all(|s| s.kind != PreeditKind::AuxCode));

    let mut router = router_with_aux(RouterConfig {
        aux_code_key: '/',
        ..RouterConfig::default()
    });
    type_letters(&mut router, "kaifa");
    let (_, _, frame) = press(&mut router, letter('/'));
    assert_eq!(preedit(&frame), "kai'fa/");
    assert!(!frame.preedit.iter().any(|s| s.kind == PreeditKind::AuxCode));
    let (_, _, frame) = press(&mut router, letter('k'));
    assert!(frame.preedit.iter().any(|s| s.kind == PreeditKind::AuxCode));
    assert!(candidate_texts(&frame).contains(&"开发"));
    let (_, _, frame) = press(&mut router, letter('h'));
    assert_eq!(candidate_texts(&frame), ["开"]);
}

/// 辅码态里敲标点：先把高亮候选上屏，再按组句外标点语义转全角。
#[test]
fn punctuation_in_aux_commits_the_highlighted_candidate_first() {
    let mut router = aux_router();
    type_letters(&mut router, "kaifa");
    press(&mut router, letter(';'));
    press(&mut router, letter('k'));
    press(&mut router, letter('f'));
    let (_, committed, frame) = press(&mut router, punct(','));
    assert_eq!(committed.as_deref(), Some("开发，"));
    assert!(frame.candidates.items.is_empty());
}

/// 码段筛空之后：空格 / 标点都不上屏原始拼音，吞掉停在辅码态等退格。
#[test]
fn an_empty_filter_swallows_space_and_punctuation() {
    let mut router = aux_router();
    type_letters(&mut router, "kaifa");
    press(&mut router, letter(';'));
    press(&mut router, letter('k'));
    let (_, _, frame) = press(&mut router, letter('q'));
    assert!(frame.candidates.items.is_empty());

    let (outcome, committed, frame) = press(&mut router, letter(' '));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(committed.is_none(), "筛空时空格不该上屏：{committed:?}");
    assert_eq!(preedit(&frame), "kai'fa;kq");

    let (_, committed, frame) = press(&mut router, punct(','));
    assert!(committed.is_none(), "筛空时标点不该上屏：{committed:?}");
    assert_eq!(preedit(&frame), "kai'fa;kq");

    let (_, _, frame) = press(&mut router, letter('1'));
    assert_eq!(preedit(&frame), "kai'fa;kq");

    press(&mut router, function_key(0x08));
    let (_, committed, frame) = press(&mut router, punct(','));
    let committed = committed.expect("有候选时标点该先上屏高亮候选");
    assert!(
        committed.starts_with("开发"),
        "上屏的是筛出来的候选：{committed}"
    );
    assert!(
        committed.ends_with('，'),
        "按组句外标点语义转全角：{committed}"
    );
    assert!(frame.candidates.items.is_empty());
}

/// Esc 在辅码态只清码段、拼音留着（不整段清掉）。
#[test]
fn escape_in_aux_only_clears_the_code() {
    let mut router = aux_router();
    type_letters(&mut router, "kaifa");
    press(&mut router, letter(';'));
    press(&mut router, letter('k'));
    press(&mut router, letter('f'));
    let (outcome, committed, frame) = press(&mut router, function_key(0x1B));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(committed.is_none());
    assert_eq!(preedit(&frame), "kai'fa");
    assert!(frame.preedit.iter().all(|s| s.kind != PreeditKind::AuxCode));
    assert!(candidate_texts(&frame).contains(&"开发"));
}

/// `shift_letter = "compose"` 下辅码态里敲大写：先清码段回拼音态（与 Esc 同语义），字母照常收进缓冲区。
#[test]
fn uppercase_with_shift_letter_compose_exits_aux_before_pushing() {
    let mut router = router_with_aux(RouterConfig {
        shift_letter_compose: true,
        ..RouterConfig::default()
    });
    type_letters(&mut router, "kaifa");
    press(&mut router, letter(';'));
    press(&mut router, letter('k'));
    let (outcome, committed, frame) = press(&mut router, letter('G'));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(committed.is_none());
    assert!(preedit(&frame).starts_with("kai'fa"));
    assert!(!preedit(&frame).contains(';'));
    assert!(frame.preedit.iter().all(|s| s.kind != PreeditKind::AuxCode));
}

/// `[general] aux_code_show` 随帧下发（候选窗按它决定要不要拼码注记）。
#[test]
fn aux_code_show_rides_the_frame() {
    let mut router = aux_router();
    let (_, _, frame) = type_letters(&mut router, "kaifa");
    assert!(!frame.aux_code_show);
    let mut router = router_with_aux(RouterConfig {
        aux_code_show: true,
        ..RouterConfig::default()
    });
    let (_, _, frame) = type_letters(&mut router, "kaifa");
    assert!(frame.aux_code_show);
}

/// 码段用完删到空之后：停在辅码态、纯拼音的候选一条不少地回来；再按一次退格才退出辅码。
#[test]
fn deleting_the_whole_code_brings_every_candidate_back() {
    let mut router = aux_router();
    let (_, _, frame) = type_letters(&mut router, "kaifa");
    let before = candidate_texts(&frame).len();
    press(&mut router, letter(';'));
    press(&mut router, letter('k'));
    press(&mut router, letter('f'));
    press(&mut router, function_key(0x08));
    let (_, _, frame) = press(&mut router, function_key(0x08));
    assert_eq!(preedit(&frame), "kai'fa;");
    assert_eq!(candidate_texts(&frame).len(), before);
    let (_, _, frame) = press(&mut router, function_key(0x08));
    assert_eq!(preedit(&frame), "kai'fa");
    assert_eq!(candidate_texts(&frame).len(), before);
}

/// 边界 14：辅码态里再敲一次触发键是幂等的——留在辅码态，既不上屏候选也不当标点。
#[test]
fn pressing_the_trigger_again_in_aux_mode_is_a_no_op() {
    let mut router = aux_router();
    type_letters(&mut router, "kaifa");
    press(&mut router, letter(';'));
    press(&mut router, letter('k'));
    let (outcome, committed, frame) = press(&mut router, letter(';'));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(committed.is_none());
    assert_eq!(preedit(&frame), "kai'fa;k");
    assert!(frame.candidates.items.iter().all(|c| c.text != "开发，"));
    let (_, _, frame) = press(&mut router, letter('f'));
    assert_eq!(preedit(&frame), "kai'fa;kf");
}

/// 老 DLL（协议 4）认不出 `AuxCode` 段：发给它那份把码段降级成普通拼音段——
/// 辅码筛选照常，只是不做淡色 + 下划线的区分。不然老 DLL 会整条消息解析失败，
/// 把码字母原样放行给应用并断开重连。
#[test]
fn an_old_dll_gets_the_code_segment_downgraded() {
    let mut router = aux_router();
    assert_eq!(
        router.handle(ClientMessage::OpenSession {
            session: SESSION,
            app: None,
            protocol: 4,
        }),
        None
    );
    type_letters(&mut router, "kaifa");
    press(&mut router, letter(';'));
    let (_, _, frame) = press(&mut router, letter('k'));
    assert_eq!(preedit(&frame), "kai'fa;k");
    assert!(
        frame.preedit.iter().all(|s| s.kind != PreeditKind::AuxCode),
        "老 DLL 不该收到 AuxCode 段：{:?}",
        frame.preedit
    );
    assert!(candidate_texts(&frame).contains(&"开发"));
    assert!(!candidate_texts(&frame).contains(&"开放"));
}

/// 降级只作用于发给 DLL 的那份：Server 自绘候选窗照常收到码段（淡色 + 下划线靠它）。
#[test]
fn the_self_drawn_window_keeps_the_code_segment_for_an_old_dll() {
    let mut router = aux_router();
    let sink = RecordingCandidates::default();
    router.set_candidate_sink(Box::new(sink.clone()));
    assert_eq!(
        router.handle(ClientMessage::OpenSession {
            session: SESSION,
            app: None,
            protocol: 4,
        }),
        None
    );
    type_letters(&mut router, "kaifa");
    let _ = router.handle(ClientMessage::PositionCandidates {
        session: SESSION,
        rect: rect(),
    });
    press(&mut router, letter(';'));
    let (_, _, frame) = press(&mut router, letter('k'));
    assert!(frame.preedit.iter().all(|s| s.kind != PreeditKind::AuxCode));
    let shown = sink.0.lock().unwrap();
    let last = shown.last().expect("自绘窗收到过帧");
    assert!(
        last.preedit.iter().any(|s| s.kind == PreeditKind::AuxCode),
        "自绘窗该看到码段：{:?}",
        last.preedit
    );
}

/// 设置页刚导入一张码表：`codes/` 目录里出现新文件就重装，不必等配置改动、也不必重启。
#[test]
fn a_new_code_table_in_the_user_dir_hot_reloads() {
    let dir = std::env::temp_dir().join(format!("qingjian-codes-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let codes = dir.join("codes");
    std::fs::create_dir_all(&codes).unwrap();
    let mut router = router();
    router.engine_mut().set_aux_enabled(true);
    router.watch_config(
        &Config::default(),
        dir.join("config.toml"),
        dir.clone(),
        DataDirs {
            user_codes: Some(codes.clone()),
            ..DataDirs::default()
        },
    );

    // 一张表都没有：总闸开着触发键也不生效（`;` 保持原生行为）
    type_letters(&mut router, "kaifa");
    let (_, _, frame) = press(&mut router, letter(';'));
    assert!(frame.preedit.iter().all(|s| s.kind != PreeditKind::AuxCode));
    press(&mut router, function_key(0x1B));

    let table = AuxCodeTable::from_pairs([("开发".to_owned(), "kf".to_owned())]).unwrap();
    table
        .write_qj(&codes.join("mine.qj"), &Default::default())
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1200));
    router.poll_config_reload();

    // 码表进来：触发键生效、逐键即筛到 开发
    type_letters(&mut router, "kaifa");
    let (_, _, frame) = press(&mut router, letter(';'));
    assert_eq!(preedit(&frame), "kai'fa;");
    let (_, _, frame) = press(&mut router, letter('k'));
    assert_eq!(candidate_texts(&frame), ["开发"]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// 同一张码表被就地改写（手工拷半个文件后补全同理）：快照按逐文件 mtime + 长度比，重装生效。
#[test]
fn a_rewritten_code_table_hot_reloads() {
    let dir = std::env::temp_dir().join(format!("qingjian-codes-rw-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let codes = dir.join("codes");
    std::fs::create_dir_all(&codes).unwrap();
    let first = AuxCodeTable::from_pairs([("开发".to_owned(), "kf".to_owned())]).unwrap();
    first
        .write_qj(&codes.join("mine.qj"), &Default::default())
        .unwrap();

    let mut router = router();
    router.engine_mut().set_aux_enabled(true);
    router.watch_config(
        &Config::default(),
        dir.join("config.toml"),
        dir.clone(),
        DataDirs {
            user_codes: Some(codes.clone()),
            ..DataDirs::default()
        },
    );
    assert!(router.engine_mut().aux_codes().is_empty());

    let rewritten = AuxCodeTable::from_pairs([("开发".to_owned(), "kh".to_owned())]).unwrap();
    rewritten
        .write_qj(&codes.join("mine.qj"), &Default::default())
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1200));
    router.poll_config_reload();
    assert_eq!(router.engine_mut().aux_codes().len(), 1);

    type_letters(&mut router, "kaifa");
    press(&mut router, letter(';'));
    let (_, _, frame) = press(&mut router, letter('k'));
    assert!(candidate_texts(&frame).contains(&"开发"));
    let (_, _, frame) = press(&mut router, letter('h'));
    assert_eq!(candidate_texts(&frame), ["开发"]);
    let _ = std::fs::remove_dir_all(&dir);
}
