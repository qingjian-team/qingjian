//! 进程级的界面语言：没设过就跟随系统；中文按简繁归成 `zh-Hans` / `zh-Hant`，其余保留系统给的标签，查表时再去掉地区。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, RwLock};

/// 系统语言取不到时用的语言。
const DEFAULT: &str = "zh-Hans";

static LOCALE: RwLock<Option<&'static str>> = RwLock::new(None);

/// 测试固定了界面语言（[`pin_locale`]）之后，别处的 [`set_locale`] 都不生效。
static PINNED: AtomicBool = AtomicBool::new(false);

/// 「看有没有固定、再写」要一口气做完：不然别的线程过了检查还没写时被固定，接着写就把固定的语言盖掉了。
static WRITING: Mutex<()> = Mutex::new(());

/// 设界面语言；空串表示跟随系统。之后所有文案表都按它查，已经画好的界面要调用方自己重画。
pub fn set_locale(tag: &str) {
    let _writing = WRITING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if PINNED.load(Ordering::Relaxed) {
        return;
    }
    store(tag);
}

/// 给测试用：把界面语言固定下来，之后读配置等处的 [`set_locale`] 不再改它。测试并行跑，比对文案的测试都固定同一种语言。
pub fn pin_locale(tag: &str) {
    let _writing = WRITING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    PINNED.store(true, Ordering::Relaxed);
    store(tag);
}

fn store(tag: &str) {
    let tag = tag.trim();
    let resolved = if tag.is_empty() {
        system_locale().unwrap_or_else(|| DEFAULT.to_owned())
    } else {
        normalize(tag)
    };
    let mut current = LOCALE
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if *current != Some(resolved.as_str()) {
        // 切语言是用户手动的事，一次进程里几次而已，泄漏几个短串换来查词时不用加锁拷贝
        *current = Some(Box::leak(resolved.into_boxed_str()));
    }
}

/// 当前界面语言；没设过就按系统语言设一次。
pub fn locale() -> &'static str {
    if let Some(tag) = *LOCALE
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
    {
        return tag;
    }
    set_locale("");
    LOCALE
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .unwrap_or(DEFAULT)
}

/// 系统的首选语言，归一化后的标签。
pub fn system_locale() -> Option<String> {
    sys_locale::get_locale().map(|tag| normalize(&tag))
}

/// `vi_VN.UTF-8` → `vi-VN`；`zh-TW`、`zh-HK`、`zh-Hant-TW` → `zh-Hant`，其余中文 → `zh-Hans`。
pub(crate) fn normalize(tag: &str) -> String {
    let tag = tag
        .split(['.', '@'])
        .next()
        .unwrap_or(tag)
        .replace('_', "-");
    let mut parts = tag.split('-');
    let language = parts.next().unwrap_or_default().to_ascii_lowercase();
    if language != "zh" {
        return tag;
    }
    let traditional = parts.any(|part| {
        ["hant", "tw", "hk", "mo"]
            .iter()
            .any(|marker| part.eq_ignore_ascii_case(marker))
    });
    if traditional { "zh-Hant" } else { "zh-Hans" }.to_owned()
}
