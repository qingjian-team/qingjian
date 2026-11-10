//! 验证上下文诊断的返回值、调用顺序、范围边界与日志隐私。
#![cfg(target_os = "macos")]

use std::cell::{Cell, RefCell};
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{AnyThread, DefinedClass, define_class, msg_send};
use objc2_foundation::{NSAttributedString, NSNotFound, NSRange, NSString};
use qingjian_core::SurroundingText;

#[allow(dead_code)]
#[path = "../src/imk/client.rs"]
mod client;

#[derive(Clone, Copy)]
struct BoundedReply {
    text: Option<&'static str>,
    actual: Option<NSRange>,
}

struct State {
    length: usize,
    selected: NSRange,
    marked: NSRange,
    replies: Vec<Option<&'static str>>,
    reads: RefCell<Vec<NSRange>>,
    bundles: Cell<usize>,
    bounded_replies: Vec<BoundedReply>,
    bounded_reads: RefCell<Vec<NSRange>>,
}

impl State {
    fn new(length: usize, selected: NSRange, replies: Vec<Option<&'static str>>) -> Self {
        Self {
            length,
            selected,
            marked: NSRange::new(NSNotFound as usize, 0),
            replies,
            reads: RefCell::new(Vec::new()),
            bundles: Cell::new(0),
            bounded_replies: vec![],
            bounded_reads: RefCell::new(Vec::new()),
        }
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "QJContextProbeTestClient"]
    #[ivars = State]
    struct FakeClient;

    impl FakeClient {
        #[unsafe(method(length))]
        fn document_length(&self) -> usize { self.ivars().length }

        #[unsafe(method(selectedRange))]
        fn selected_range(&self) -> NSRange { self.ivars().selected }

        #[unsafe(method(markedRange))]
        fn marked_range(&self) -> NSRange { self.ivars().marked }

        #[unsafe(method_id(attributedSubstringFromRange:))]
        fn substring(&self, range: NSRange) -> Option<Retained<NSAttributedString>> {
            let state = self.ivars();
            let index = state.reads.borrow().len();
            state.reads.borrow_mut().push(range);
            state.replies[index].map(|text| {
                NSAttributedString::initWithString(NSAttributedString::alloc(), &NSString::from_str(text))
            })
        }

        #[unsafe(method_id(stringFromRange:actualRange:))]
        unsafe fn bounded_substring(&self, range: NSRange, actual: *mut NSRange) -> Option<Retained<NSString>> {
            let state = self.ivars();
            let index = state.bounded_reads.borrow().len();
            state.bounded_reads.borrow_mut().push(range);
            let reply = state.bounded_replies.get(index).copied()
                .unwrap_or(BoundedReply { text: None, actual: None });
            if let Some(range) = reply.actual { unsafe { *actual = range; } }
            reply.text.map(NSString::from_str)
        }

        #[unsafe(method_id(bundleIdentifier))]
        fn bundle_identifier(&self) -> Retained<NSString> {
            self.ivars().bundles.set(self.ivars().bundles.get() + 1);
            NSString::from_str("test.context-probe")
        }
    }
);

struct Capture(Arc<Mutex<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn run(
    state: State,
    before: usize,
    after: usize,
    debug: bool,
) -> (
    Option<SurroundingText>,
    Vec<NSRange>,
    String,
    usize,
    Vec<NSRange>,
) {
    let fake: Retained<FakeClient> =
        unsafe { msg_send![super(FakeClient::alloc().set_ivars(state)), init] };
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let writer = buffer.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(if debug {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .without_time()
        .with_ansi(false)
        .with_writer(move || Capture(writer.clone()))
        .finish();
    let result = tracing::subscriber::with_default(subscriber, || {
        client::TextClient::new(&fake).surrounding_text(before, after, "test_phase")
    });
    let reads = fake.ivars().reads.borrow().clone();
    let logs = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
    let bounded = fake.ivars().bounded_reads.borrow().clone();
    (result, reads, logs, fake.ivars().bundles.get(), bounded)
}

#[test]
fn unavailable_selection_and_zero_length_skip_substrings() {
    for (length, selection, outcome) in [
        (
            10,
            NSRange::new(NSNotFound as usize, 0),
            "selection_unavailable",
        ),
        (0, NSRange::new(0, 0), "zero_document_length"),
    ] {
        let (result, reads, logs, bundles, bounded) =
            run(State::new(length, selection, vec![]), 4, 4, true);
        assert!(result.is_none());
        assert!(reads.is_empty());
        assert!(logs.contains(outcome));
        assert_eq!(bundles, 1);
        assert_eq!(bounded.len(), usize::from(length == 0));
    }
}

#[test]
fn before_nil_shortcircuits_and_after_nil_discards_before() {
    let state = State::new(10, NSRange::new(5, 0), vec![None]);
    let (result, reads, logs, _, _) = run(state, 3, 2, true);
    assert!(result.is_none());
    assert_eq!(reads, vec![NSRange::new(2, 3)]);
    assert!(logs.contains("outcome=\"before_nil\""));
    assert!(logs.contains("after_status=\"not_attempted\""));

    let state = State::new(10, NSRange::new(5, 0), vec![Some("SECRET_PRECEDING"), None]);
    let (result, reads, logs, _, _) = run(state, 3, 2, true);
    assert!(result.is_none());
    assert_eq!(reads, vec![NSRange::new(2, 3), NSRange::new(5, 2)]);
    assert!(logs.contains("outcome=\"after_nil\""));
    assert!(!logs.contains("SECRET_PRECEDING"));
}

#[test]
fn empty_ranges_skip_substrings_and_return_empty_strings() {
    let (result, reads, logs, _, bounded) =
        run(State::new(1, NSRange::new(0, 0), vec![]), 0, 0, true);
    assert!(bounded.is_empty());
    let result = result.unwrap();
    assert_eq!((result.before.as_str(), result.after.as_str()), ("", ""));
    assert!(reads.is_empty());
    assert!(logs.contains("before_status=\"empty_range\""));
    assert!(logs.contains("after_status=\"empty_range\""));
}

#[test]
fn marked_boundaries_utf16_lengths_and_content_privacy() {
    const SECRET: &str = "SECRET😀";
    let mut state = State::new(20, NSRange::new(9, 0), vec![Some(SECRET), Some("后文")]);
    state.marked = NSRange::new(8, 3);
    let (result, reads, logs, _, bounded) = run(state, 4, 5, true);
    assert!(bounded.is_empty());
    let result = result.unwrap();
    assert_eq!(result.before, SECRET);
    assert_eq!(result.after, "后文");
    assert_eq!(reads, vec![NSRange::new(4, 4), NSRange::new(11, 5)]);
    assert!(logs.contains("before_utf16=Some(8)"));
    assert!(logs.contains("after_utf16=Some(2)"));
    assert!(logs.contains("test.context-probe"));
    assert!(logs.contains("test_phase"));
    assert!(!logs.contains(SECRET));
    assert!(!logs.contains("后文"));
}

#[test]
fn info_level_preserves_result_without_diagnostic_client_calls() {
    let state = State::new(10, NSRange::new(5, 0), vec![Some("private"), Some("tail")]);
    let (result, reads, logs, bundles, bounded) = run(state, 3, 2, false);
    assert_eq!(result.unwrap().before, "private");
    assert_eq!(reads.len(), 2);
    assert!(logs.is_empty());
    assert_eq!(bundles, 0);
    assert!(bounded.is_empty());
}

fn bounded_state() -> State {
    let mut state = State::new(0, NSRange::new(4, 0), vec![]);
    state.marked = NSRange::new(2, 0); // A stale empty marked range must not move the anchor.
    state
}

fn reply(text: Option<&'static str>, actual: Option<NSRange>) -> BoundedReply {
    BoundedReply { text, actual }
}

fn side<'a>(logs: &'a str, name: &str) -> &'a str {
    logs.lines()
        .find(|line| {
            line.contains("bounded surrounding context probe")
                && line.contains(&format!("side=\"{name}\""))
        })
        .unwrap()
}

#[test]
fn bounded_zero_length_uses_selection_and_logs_utf16_without_content() {
    let mut state = bounded_state();
    state.bounded_replies = vec![
        reply(Some("秘😀密"), Some(NSRange::new(0, 4))),
        reply(Some("SYNTHETIC_SECRET"), Some(NSRange::new(4, 16))),
    ];
    let (result, legacy, logs, _, bounded) = run(state, 64, 32, true);
    assert!(result.is_none());
    assert!(legacy.is_empty());
    assert_eq!(bounded, vec![NSRange::new(0, 4), NSRange::new(4, 32)]);
    let before = side(&logs, "before");
    assert!(before.contains("anchor_source=\"selection\""));
    assert!(before.contains("returned_utf16=Some(4)"));
    assert!(before.contains(&format!("actual={:?}", NSRange::new(0, 4))));
    for key in [
        "actual_range_valid",
        "length_matches",
        "within_request",
        "touches_anchor",
    ] {
        assert!(before.contains(&format!("{key}=true")), "{before}");
    }
    assert!(side(&logs, "after").contains("returned_utf16=Some(16)"));
    assert!(!logs.contains("秘😀密"));
    assert!(!logs.contains("SYNTHETIC_SECRET"));
}

#[test]
fn bounded_rejects_unwritten_misaligned_and_wrong_length_actual_ranges() {
    for (actual, flag) in [
        (None, "actual_range_valid=false"),
        (Some(NSRange::new(0, 3)), "touches_anchor=false"),
        (Some(NSRange::new(0, 4)), "length_matches=false"),
        (Some(NSRange::new(0, 5)), "within_request=false"),
    ] {
        let mut state = bounded_state();
        state.bounded_replies = vec![reply(Some("abc"), actual)];
        let (result, _, logs, _, reads) = run(state, 64, 32, true);
        assert!(result.is_none());
        assert_eq!(reads.len(), 2);
        assert!(side(&logs, "before").contains(flag), "{logs}");
    }
}

#[test]
fn bounded_before_nil_still_reads_after_and_preserves_none() {
    let mut state = bounded_state();
    state.bounded_replies = vec![
        reply(None, None),
        reply(Some("x"), Some(NSRange::new(4, 1))),
    ];
    let (result, legacy, logs, _, reads) = run(state, 64, 32, true);
    assert!(result.is_none());
    assert!(legacy.is_empty());
    assert_eq!(reads.len(), 2);
    assert!(side(&logs, "before").contains("status=\"nil\""));
    assert!(side(&logs, "after").contains("status=\"value\""));
}

#[test]
fn bounded_positive_marked_anchor_and_hard_caps() {
    let mut state = State::new(0, NSRange::new(120, 0), vec![]);
    state.marked = NSRange::new(80, 3);
    let (result, legacy, logs, _, reads) = run(state, 192, 192, true);
    assert!(result.is_none());
    assert!(legacy.is_empty());
    assert_eq!(reads, vec![NSRange::new(16, 64), NSRange::new(83, 32)]);
    assert!(side(&logs, "before").contains("anchor_source=\"marked\""));
}

#[test]
fn bounded_invalid_anchors_skip_all_requests() {
    for selected in [
        NSRange::new(NSNotFound as usize, 0),
        NSRange::new(usize::MAX - 1, 4),
        NSRange::new(4, NSNotFound as usize),
        NSRange::new(NSNotFound as usize - 1, 1),
    ] {
        let state = State::new(0, selected, vec![]);
        let (result, legacy, logs, _, bounded) = run(state, 64, 32, true);
        assert!(result.is_none());
        assert!(legacy.is_empty() && bounded.is_empty());
        assert!(logs.contains("outcome=\"invalid_anchor\""));
    }
}

#[test]
fn bounded_zero_length_info_is_disabled_and_unknown_length_keeps_legacy_none() {
    let (result, legacy, logs, bundles, bounded) = run(bounded_state(), 64, 32, false);
    assert!(result.is_none());
    assert!(legacy.is_empty() && bounded.is_empty() && logs.is_empty());
    assert_eq!(bundles, 0);
    let state = State::new(NSNotFound as usize, NSRange::new(4, 0), vec![None]);
    let (result, legacy, logs, _, bounded) = run(state, 64, 32, true);
    assert!(result.is_none());
    assert_eq!(legacy, vec![NSRange::new(0, 4)]);
    assert_eq!(bounded, vec![NSRange::new(0, 4), NSRange::new(4, 32)]);
    assert!(logs.contains("length_unknown=true"));
}
