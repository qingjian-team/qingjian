//! 辅码在壳里的闭环：触发与逐键加权、帧上的码段、码段对不上也不筛空与退格边界、
//! 标点先上屏、码表热加载、老 DLL 降级，以及自绘窗与 DLL 面各看各的帧。

use std::sync::Arc;

use qingjian_dictionary::{AuxCodeLookup, AuxCodeTable};
use qingjian_platform::protocol::PreeditKind;
use qingjian_platform::{Config, UserDirs};
use qingjian_windows_server::dispatch::DataDirs;

use super::support::*;

/// 装了辅码码表的 Router：`kaifa` 之后敲触发键进辅码态、`kf` 把 开发 排到第一。
/// 码表由 Core 的 [`AuxCodeLookup`] 注入；辅码总闸缺省关，这里显式打开。
fn router_with_aux(config: RouterConfig) -> Router {
    // 这些用例写的是「码段只调顺序、不筛候选」的语义，`[general] aux_code_weighting` 缺省是关的
    router_with_aux_weighting(config, true)
}

/// 码表 + 辅码总闸开着，加权开关由调用方决定（缺省关闭的路径见 `aux_code_weighting_flips_with_config`）。
fn router_with_aux_weighting(config: RouterConfig, weighting: bool) -> Router {
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
    engine.set_aux_weighting(weighting);
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

/// 辅码态：触发键进状态、码段按 AuxCode 段下发、逐键加权（对得上的往前排，一条都不筛掉）、
/// 退格逐级放宽。
#[test]
fn aux_trigger_ranks_the_code_segment_without_filtering() {
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
    // 码段是权重不是筛子：对得上的往前排，对不上的（开放）留在候选里
    let texts = candidate_texts(&frame);
    assert_eq!(texts.len(), before, "码段不该筛掉候选：{texts:?}");
    assert!(texts.contains(&"开放"));

    let (_, _, frame) = press(&mut router, letter('f'));
    assert_eq!(candidate_texts(&frame)[0], "开发");
    assert_eq!(candidate_texts(&frame).len(), before);
    assert_eq!(frame.candidates.items[0].aux_code.as_deref(), Some("kf"));

    // 码段对不上（`kfq`）：一条候选都不少，也不吐原始拼音
    let (_, _, frame) = press(&mut router, letter('q'));
    assert_eq!(preedit(&frame), "kai'fa;kfq");
    let texts = candidate_texts(&frame);
    assert_eq!(texts.len(), before, "码段对不上也不该筛空：{texts:?}");
    assert!(texts.contains(&"开发"));
    assert!(!frame.is_empty());

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
    let (_, _, frame) = type_letters(&mut router, "kaifa");
    let before = candidate_texts(&frame).len();
    let (_, _, frame) = press(&mut router, letter('/'));
    assert_eq!(preedit(&frame), "kai'fa/");
    assert!(!frame.preedit.iter().any(|s| s.kind == PreeditKind::AuxCode));
    let (_, _, frame) = press(&mut router, letter('k'));
    assert!(frame.preedit.iter().any(|s| s.kind == PreeditKind::AuxCode));
    assert_eq!(candidate_texts(&frame).len(), before);
    let (_, _, frame) = press(&mut router, letter('h'));
    // 配置的触发键与码表都生效：`kh` 完全匹配的 开 排第一，剩下的照旧在
    assert_eq!(candidate_texts(&frame)[0], "开");
    assert_eq!(candidate_texts(&frame).len(), before);
    assert!(candidate_texts(&frame).contains(&"开放"));
}

/// `[general] aux_code_weighting` 从配置文件接到引擎：缺省关着时码段是过滤器（对不上码的词不出），
/// 写进配置文件并热加载后变成只调顺序、一条都不筛。
#[test]
fn aux_code_weighting_flips_with_config() {
    let dir = std::env::temp_dir().join(format!("qingjian-aux-weight-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let config_path = dir.join("config.toml");

    let mut router = router_with_aux_weighting(RouterConfig::default(), false);
    router.watch_config(
        &Config::default(),
        config_path.clone(),
        dir.clone(),
        DataDirs {
            user: Some(UserDirs::at(dir.clone())),
            ..DataDirs::default()
        },
    );
    // 缺省：码段是过滤器，`kf` 之外的候选不出
    type_letters(&mut router, "kaifa");
    press(&mut router, letter(';'));
    let (_, _, frame) = press(&mut router, letter('k'));
    let filtered = candidate_texts(&frame);
    assert!(
        !filtered.contains(&"开放"),
        "缺省码段是过滤器：{filtered:?}"
    );
    // Esc 一下清码段、再一下清拼音，下一段输入从干净的组句开始
    press(&mut router, function_key(0x1B));
    let (_, _, frame) = press(&mut router, function_key(0x1B));
    assert!(
        preedit(&frame).is_empty(),
        "组句该清空：{:?}",
        preedit(&frame)
    );

    // 配置文件里打开加权：热加载后同一串键不再筛候选。热加载会按配置文件重设辅码总闸
    // （`[aux_code] enabled` 缺省 false），这条用例只看加权开关，重装码表后把总闸再打开一次。
    std::fs::write(&config_path, "[general]\naux_code_weighting = true\n").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1200));
    router.poll_config_reload();
    router.engine_mut().set_aux_enabled(true);

    let (_, _, frame) = type_letters(&mut router, "kaifa");
    let before = candidate_texts(&frame).len();
    press(&mut router, letter(';'));
    let (_, _, frame) = press(&mut router, letter('k'));
    assert_eq!(
        candidate_texts(&frame).len(),
        before,
        "加权打开后码段不该筛掉候选：{:?}",
        candidate_texts(&frame)
    );
    assert!(candidate_texts(&frame).contains(&"开放"));
    let _ = std::fs::remove_dir_all(&dir);
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

/// 一条候选都没有时（拼音切得动、词库没词）：辅码态里空格 / 标点 / 数字都不上屏原始拼音，
/// 吞掉停在辅码态等退格。码段本身不再筛空，这条兜住的是「拼音本来就没候选」那种边界。
#[test]
fn an_empty_candidate_list_swallows_space_and_punctuation() {
    let mut router = aux_router();
    let (_, _, frame) = type_letters(&mut router, "cuan");
    assert!(
        frame.candidates.items.is_empty(),
        "用例前提：cuan 在样例词库里没有词"
    );
    let (outcome, committed, frame) = press(&mut router, letter(';'));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(committed.is_none());
    assert_eq!(preedit(&frame), "cuan;");
    press(&mut router, letter('k'));
    let (_, _, frame) = press(&mut router, letter('q'));
    assert!(frame.candidates.items.is_empty());
    assert_eq!(preedit(&frame), "cuan;kq");

    let (outcome, committed, frame) = press(&mut router, letter(' '));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(committed.is_none(), "没候选时空格不该上屏：{committed:?}");
    assert_eq!(preedit(&frame), "cuan;kq");

    let (_, committed, frame) = press(&mut router, punct(','));
    assert!(committed.is_none(), "没候选时标点不该上屏：{committed:?}");
    assert_eq!(preedit(&frame), "cuan;kq");

    let (_, _, frame) = press(&mut router, letter('1'));
    assert_eq!(preedit(&frame), "cuan;kq");

    // 退格把码段删空：还是辅码态，`;` 仍留着
    press(&mut router, function_key(0x08));
    let (_, _, frame) = press(&mut router, function_key(0x08));
    assert_eq!(preedit(&frame), "cuan;");
    // 再一记退格才退出辅码，拼音一个字都不动
    let (_, _, frame) = press(&mut router, function_key(0x08));
    assert_eq!(preedit(&frame), "cuan");
    assert!(frame.preedit.iter().all(|s| s.kind != PreeditKind::AuxCode));
}

/// 组句中按住 Ctrl 敲触发键也进辅码态（有些布局里 `;` 只能带着 Ctrl 送到，也有人习惯按住）。
/// 判定走 `KeyModifiers`，引擎那边还是同一个 `enter_aux`，没有为 Ctrl 单开入口。
#[test]
fn ctrl_with_the_trigger_key_enters_aux_mode() {
    let mut router = aux_router();
    let (_, _, frame) = type_letters(&mut router, "kaifa");
    let before = candidate_texts(&frame).len();
    let (outcome, committed, frame) = press(
        &mut router,
        letter_with(
            ';',
            KeyModifiers {
                ctrl: true,
                ..KeyModifiers::default()
            },
        ),
    );
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(committed.is_none());
    assert_eq!(preedit(&frame), "kai'fa;");
    assert_eq!(candidate_texts(&frame).len(), before);
    // 之后照常逐键加权
    let (_, _, frame) = press(&mut router, letter('f'));
    assert_eq!(preedit(&frame), "kai'fa;f");
}

/// 辅码态里按住 Ctrl 敲触发键同样是幂等的：既不退出也不当标点。
#[test]
fn ctrl_with_the_trigger_key_stays_put_in_aux_mode() {
    let mut router = aux_router();
    type_letters(&mut router, "kaifa");
    press(&mut router, letter(';'));
    press(&mut router, letter('k'));
    let (outcome, committed, frame) = press(
        &mut router,
        letter_with(
            ';',
            KeyModifiers {
                ctrl: true,
                ..KeyModifiers::default()
            },
        ),
    );
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(committed.is_none());
    assert_eq!(preedit(&frame), "kai'fa;k");
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

/// 辅码态里再敲一次触发键是幂等的——留在辅码态，既不上屏候选也不当标点。
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
    let (_, _, frame) = type_letters(&mut router, "kaifa");
    let before = candidate_texts(&frame).len();
    press(&mut router, letter(';'));
    let (_, _, frame) = press(&mut router, letter('k'));
    assert_eq!(preedit(&frame), "kai'fa;k");
    assert!(
        frame.preedit.iter().all(|s| s.kind != PreeditKind::AuxCode),
        "老 DLL 不该收到 AuxCode 段：{:?}",
        frame.preedit
    );
    // 降级的只是码段的样式，加权照旧：一条候选都不少
    assert_eq!(candidate_texts(&frame).len(), before);
    assert!(candidate_texts(&frame).contains(&"开发"));
    assert!(candidate_texts(&frame).contains(&"开放"));
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

/// 设置页刚导入一张码表：`custom/codes/` 目录里出现新文件就重装，不必等配置改动、也不必重启。
#[test]
fn a_new_code_table_in_the_user_dir_hot_reloads() {
    let dir = std::env::temp_dir().join(format!("qingjian-codes-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let codes = dir.join("custom/codes");
    std::fs::create_dir_all(&codes).unwrap();
    let mut router = router();
    router.engine_mut().set_aux_enabled(true);
    router.watch_config(
        &Config::default(),
        dir.join("config.toml"),
        dir.clone(),
        DataDirs {
            user: Some(UserDirs::at(dir.clone())),
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

    // 码表进来：触发键生效、逐键加权把 开发 排到第一
    type_letters(&mut router, "kaifa");
    let (_, _, frame) = press(&mut router, letter(';'));
    assert_eq!(preedit(&frame), "kai'fa;");
    let (_, _, frame) = press(&mut router, letter('k'));
    assert_eq!(candidate_texts(&frame)[0], "开发");
    assert_eq!(frame.candidates.items[0].aux_code.as_deref(), Some("kf"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// 同一张码表被就地改写（手工拷半个文件后补全同理）：快照按逐文件 mtime + 长度比，重装生效。
#[test]
fn a_rewritten_code_table_hot_reloads() {
    let dir = std::env::temp_dir().join(format!("qingjian-codes-rw-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let codes = dir.join("custom/codes");
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
            user: Some(UserDirs::at(dir.clone())),
            ..DataDirs::default()
        },
    );
    assert!(router.engine_mut().aux_codes().is_empty());

    // 快照按「逐文件 mtime + 长度」比：改写后长度没变（kf → kh），时间戳要是与快照落在同一个
    // 时钟刻度上就看不出改动 —— 先等一格过去，免得这条用例时好时坏（Windows 上实测会撞）。
    std::thread::sleep(std::time::Duration::from_millis(50));
    let rewritten = AuxCodeTable::from_pairs([("开发".to_owned(), "kh".to_owned())]).unwrap();
    rewritten
        .write_qj(&codes.join("mine.qj"), &Default::default())
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1200));
    router.poll_config_reload();
    assert_eq!(router.engine_mut().aux_codes().len(), 1);

    type_letters(&mut router, "kaifa");
    press(&mut router, letter(';'));
    // 改后的码表（开发 = kh）生效：`k` 只把命中前缀的往前挪，`kh` 完全匹配后第一就是 开发
    press(&mut router, letter('k'));
    let (_, _, frame) = press(&mut router, letter('h'));
    assert_eq!(candidate_texts(&frame)[0], "开发");
    assert_eq!(frame.candidates.items[0].aux_code.as_deref(), Some("kh"));
    let _ = std::fs::remove_dir_all(&dir);
}
