//! C ABI 边界：与 fcitx5-shim/qingjian.h 一一对应，业务全在 [`crate::host`]。
//!
//! 线程模型：fcitx5 的引擎回调都在主循环单线程里，与 macOS 壳同款用 thread_local 存 Host。
//! 字符串返回约定：`*const c_char` 指向 thread_local 缓冲，**只在下一次 qj_ 调用前有效**,
//! C++ 侧必须当场拷进 std::string。

use std::cell::RefCell;
use std::ffi::{CStr, CString, c_char, c_int, c_uint};

use crate::host::Host;

thread_local! {
    static HOST: RefCell<Option<Host>> = const { RefCell::new(None) };
    static BUF: RefCell<CString> = RefCell::new(CString::default());
}

/// 在 Host 上跑一段逻辑，**捕获 panic**:extern "C" 里 panic 越过 FFI 边界会 abort 掉
/// 整个 fcitx5 进程（所有输入法一起崩）。捕获后记日志、返回 None，当次按键退化成透传。
/// 与 macOS 壳的 `catch_panic` 同一防线。
fn with_host<T>(f: impl FnOnce(&mut Host) -> T) -> Option<T> {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        HOST.with(|host| host.borrow_mut().as_mut().map(f))
    }));
    match result {
        Ok(value) => value,
        Err(_) => {
            tracing::error!("引擎调用 panic 已被 FFI 边界捕获,本次按键透传");
            None
        }
    }
}

/// 把字符串放进 thread_local 缓冲并返回指针（内部 NUL 替换成空格，防 panic）。
fn hand_out(s: &str) -> *const c_char {
    let cleaned;
    let bytes = if s.as_bytes().contains(&0) {
        cleaned = s.replace('\0', " ");
        cleaned.as_bytes()
    } else {
        s.as_bytes()
    };
    BUF.with(|buf| {
        *buf.borrow_mut() = CString::new(bytes).expect("NUL 已清除");
        buf.borrow().as_ptr()
    })
}

static VERSION: &CStr = c"qingjian-linux 0.1.0";

#[unsafe(no_mangle)]
pub extern "C" fn qj_version() -> *const c_char {
    VERSION.as_ptr()
}

/// 装配引擎，读 $XDG_DATA_HOME/qingjian（缺省 ~/.local/share/qingjian）下的数据文件。
/// 返回 false = 数据缺失或加载失败，错误说明由 `qj_init_error` 取。
#[unsafe(no_mangle)]
pub extern "C" fn qj_init() -> bool {
    // 日志落文件(~/.local/state/qingjian/logs);guard 活到进程结束，丢了会吞掉日志尾巴。
    static LOG_GUARD: std::sync::OnceLock<Option<tracing_appender::non_blocking::WorkerGuard>> =
        std::sync::OnceLock::new();
    LOG_GUARD.get_or_init(crate::logging::init);
    let Some(dir) = crate::host::data_dir() else {
        return set_init_error("HOME/XDG_DATA_HOME 都不在,找不到数据目录".to_owned());
    };
    // Host::init 里加载 mmap 零拷贝 .qj 词库：文件被截断/损坏会 slice 越界 panic。
    // 同样必须挡在 FFI 边界内（否则 fcitx5 崩溃循环，用户从 UI 无法恢复）。
    let built = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| Host::init(dir, None)));
    let host = match built {
        Ok(Ok(host)) => host,
        Ok(Err(message)) => return set_init_error(message),
        Err(_) => return set_init_error("数据文件损坏或格式错误,装配时 panic(已捕获)".to_owned()),
    };
    let mut host = host;
    // 配置热加载：改 ~/.config/qingjian/config.toml，敲下一个键即生效。
    if let Some(path) = crate::host::config_path() {
        host.watch_config(path);
    }
    HOST.with(|slot| *slot.borrow_mut() = Some(host));
    true
}

thread_local! {
    static INIT_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

fn set_init_error(message: String) -> bool {
    INIT_ERROR.with(|e| *e.borrow_mut() = message);
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn qj_init_error() -> *const c_char {
    INIT_ERROR.with(|e| hand_out(&e.borrow()))
}

/// 按键事件。返回 true = 吞掉（shim 调 filterAndAccept 并随后取状态）。
#[unsafe(no_mangle)]
pub extern "C" fn qj_key_event(keyval: c_uint, state: c_uint, release: bool) -> bool {
    with_host(|h| h.key(keyval, state, release)).unwrap_or(false)
}

/// 取走待上屏文本；没有返回 NULL。
#[unsafe(no_mangle)]
pub extern "C" fn qj_take_commit() -> *const c_char {
    match with_host(|h| h.pending_commit.take()).flatten() {
        Some(text) => hand_out(&text),
        None => std::ptr::null(),
    }
}

/// 拼音行（空串 = 没在组句，收窗）。
#[unsafe(no_mangle)]
pub extern "C" fn qj_preedit() -> *const c_char {
    match with_host(|h| h.preedit.clone()) {
        Some(text) => hand_out(&text),
        None => hand_out(""),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn qj_preedit_cursor() -> c_int {
    with_host(|h| h.preedit_cursor as c_int).unwrap_or(0)
}

/// 当前页候选数（页内下标 0..count）。
#[unsafe(no_mangle)]
pub extern "C" fn qj_candidate_count() -> c_int {
    with_host(|h| {
        let start = h.page * h.layout.page_size();
        h.layout
            .len()
            .saturating_sub(start)
            .min(h.layout.page_size()) as c_int
    })
    .unwrap_or(0)
}

fn page_candidate<T>(offset: c_int, f: impl FnOnce(&qingjian_core::Candidate) -> T) -> Option<T> {
    with_host(|h| {
        let index = h.page * h.layout.page_size() + offset as usize;
        h.layout.candidate(index).map(f)
    })
    .flatten()
}

#[unsafe(no_mangle)]
pub extern "C" fn qj_candidate_text(offset: c_int) -> *const c_char {
    match page_candidate(offset, |c| c.text.clone()) {
        Some(text) => hand_out(&text),
        None => hand_out(""),
    }
}

/// 候选的译文（学习语言；无译文返回空串）。脚手架期由 fcitx5 面板当 comment 显示。
#[unsafe(no_mangle)]
pub extern "C" fn qj_candidate_comment(offset: c_int) -> *const c_char {
    let comment = page_candidate(offset, |c| {
        c.translation
            .as_ref()
            .and_then(|t| t.senses().first())
            .map(|s| s.text.clone())
            .unwrap_or_default()
    });
    hand_out(&comment.unwrap_or_default())
}

/// 页码指示 "1/28"（空串 = 只有一页，不显示）。
#[unsafe(no_mangle)]
pub extern "C" fn qj_page_indicator() -> *const c_char {
    match with_host(|h| h.page_indicator()) {
        Some(text) => hand_out(&text),
        None => hand_out(""),
    }
}

/// 页内高亮下标；-1 = 无。
#[unsafe(no_mangle)]
pub extern "C" fn qj_highlight() -> c_int {
    with_host(|h| {
        if h.layout.is_empty() {
            return -1;
        }
        let start = h.page * h.layout.page_size();
        if h.highlighted >= start && h.highlighted < start + h.layout.page_size() {
            (h.highlighted - start) as c_int
        } else {
            -1
        }
    })
    .unwrap_or(-1)
}

/// 私密输入（密码框等）：true = 学习/日志静音。变了才需要调。
#[unsafe(no_mangle)]
pub extern "C" fn qj_set_private(private: bool) {
    with_host(|h| h.set_private(private));
}

/// 上屏当前页第 `offset` 格（鼠标点选）。
#[unsafe(no_mangle)]
pub extern "C" fn qj_select(offset: c_int) {
    with_host(|h| h.select_on_page(offset.max(0) as usize));
}

/// 当前应用变了（fcitx5 的 program 名）：按应用关英文候选用。shim 在名字变化时才调。
#[unsafe(no_mangle)]
pub extern "C" fn qj_set_program(program: *const c_char) {
    if program.is_null() {
        return;
    }
    let name = unsafe { CStr::from_ptr(program) }
        .to_string_lossy()
        .into_owned();
    with_host(|h| h.set_program(&name));
}

/// 拼音行显示位置（配置 `[general] preedit`）：0 = 行内+窗口，1 = 只行内，2 = 只窗口。
#[unsafe(no_mangle)]
pub extern "C" fn qj_preedit_display() -> c_uint {
    with_host(|h| h.preedit_display()).unwrap_or(0)
}

/// 本地整句模型的定时驱动（shim 的 fcitx5 TimeEvent 每 20ms 调一次）。
/// 返回位掩码：bit0 = 排序变了，shim 要重画面板；bit1 = 还有事在等，继续定时。
#[unsafe(no_mangle)]
pub extern "C" fn qj_model_poll() -> c_uint {
    with_host(|h| h.model_poll()).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn qj_focus_in() {}

#[unsafe(no_mangle)]
pub extern "C" fn qj_focus_out() {
    with_host(|h| h.reset());
}

#[unsafe(no_mangle)]
pub extern "C" fn qj_reset() {
    with_host(|h| h.reset());
}
