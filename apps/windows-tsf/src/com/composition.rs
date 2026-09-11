//! 组句 preedit：把 Server 回来的拼音行经 TSF 组句（[`ITfComposition`]）显示在文档光标处。
//!
//! 对应 macOS 端的「内联 marked text」（`setMarkedText`）：敲字时未上屏的拼音带下划线显示在光标处，
//! 告诉应用「正在这里组字」。富样式的拼音行（纠错删除线、光标后淡色）在候选窗口里另画（见候选窗口模块），
//! 这里只放最朴素的一行拼音——与 mac 内联那份一致（不含被纠错划掉的原字母）。
//!
//! 所有写操作都在**异步读写编辑会话**里做（理由见 [`super::edit_session`]）。组句对象要跨按键存活，
//! 放在 [`Shared`] 里由 `TextService`、编辑会话、组句 sink 共享（STA 单线程，用 `Rc` + `RefCell`/`Cell`）。

use std::cell::{Cell, RefCell};
use std::mem::ManuallyDrop;
use std::rc::Rc;

use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::UI::TextServices::{
    INSERT_TEXT_AT_SELECTION_FLAGS, ITfComposition, ITfCompositionSink, ITfCompositionSink_Impl,
    ITfContext, ITfContextComposition, ITfInsertAtSelection, ITfRange, TF_AE_END, TF_ANCHOR_END,
    TF_IAS_QUERYONLY, TF_SELECTION, TF_SELECTIONSTYLE,
};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
use windows::core::{BOOL, Interface, Ref, Result, implement};

use qingjian_platform::protocol::{Frame, PreeditKind};

use super::candidates::CandidateWindow;

/// `TextService` 与编辑会话、组句 sink 之间共享的组句状态。单线程（TSF 是 STA），用 `Rc` 传递。
pub(crate) struct Shared {
    /// 当前活动的组句；`None` 表示没在组句。跨按键存活。
    composition: RefCell<Option<ITfComposition>>,

    /// 组句影子标志：Server 每次回的 frame 空不空，决定 `OnTestKeyDown` 要不要吃功能键。
    composing: Cell<bool>,

    /// 候选窗口（`Activate` 时建，`Deactivate` 时销）。建失败时为 `None`，降级为无候选 UI。
    window: RefCell<Option<CandidateWindow>>,
}

impl Shared {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self {
            composition: RefCell::new(None),
            composing: Cell::new(false),
            window: RefCell::new(None),
        })
    }

    /// 是否正在组句（`OnTestKeyDown` 用）。
    pub(crate) fn composing(&self) -> bool {
        self.composing.get()
    }

    /// 是否还有一个活动组句句柄没收（`update_document` 用来判断要不要跑编辑会话把它收掉）。
    pub(crate) fn has_composition(&self) -> bool {
        self.composition.borrow().is_some()
    }

    /// 更新组句影子标志。
    pub(crate) fn set_composing(&self, value: bool) {
        self.composing.set(value);
    }

    /// 装 / 卸候选窗口。传 `None` 会析构原窗口（销毁 HWND）。
    pub(crate) fn set_window(&self, window: Option<CandidateWindow>) {
        *self.window.borrow_mut() = window;
    }

    /// 用一帧刷新候选窗口内容（不定位、不显示；定位与显示在编辑会话里做，那时才有光标位置）。
    pub(crate) fn update_candidates(&self, frame: &Frame) {
        if let Some(window) = self.window.borrow().as_ref() {
            window.set_content(frame);
        }
    }

    /// 清掉一切组句状态（停用 / 组句被应用强行终止时）：丢组句句柄、收起候选窗口。不碰文档。
    pub(crate) fn reset(&self) {
        *self.composition.borrow_mut() = None;
        self.composing.set(false);
        if let Some(window) = self.window.borrow().as_ref() {
            window.hide();
        }
    }
}

/// 从一帧里拼出内联要显示的拼音行：按顺序拼各段文本，**跳过被纠错划掉的原字母**（那些只在候选窗里
/// 画删除线）。空串表示这一帧没有组句内容。
pub(crate) fn preedit_string(frame: &Frame) -> String {
    frame
        .preedit
        .iter()
        .filter(|segment| segment.kind != PreeditKind::Corrected)
        .map(|segment| segment.text.as_str())
        .collect()
}

/// 按本次按键算出的「上屏文本 + 组句拼音行」把文档更新到目标状态。在编辑会话回调（持写锁 `ec`）里调。
///
/// 状态机：先落定要上屏的 `commit`（有组句就替换组句范围再结束、否则选区插入），再按 `preedit`
/// 起 / 改 / 收组句。`shared` 用 `&Rc` 是因为起新组句要把它克隆进 [`CompositionSink`]。
pub(crate) fn apply(
    shared: &Rc<Shared>,
    context: &ITfContext,
    ec: u32,
    commit: Option<&str>,
    preedit: &str,
) -> Result<()> {
    if let Some(text) = commit {
        commit_text(shared, context, ec, text)?;
    }
    if preedit.is_empty() {
        end_composition(shared, ec)?;
    } else {
        update_preedit(shared, context, ec, preedit)?;
    }
    // 组句更新完，顺便按光标位置摆好候选窗口（此刻有 ec 和组句范围，能拿到光标屏幕矩形）。
    update_window(shared, context, ec);
    Ok(())
}

/// 组句在进行就把候选窗口摆到光标下方显示，否则收起。内容（候选行）已在收键时由 `update_candidates` 设好。
fn update_window(shared: &Rc<Shared>, context: &ITfContext, ec: u32) {
    let window = shared.window.borrow();
    let Some(window) = window.as_ref() else {
        return;
    };
    let active = shared.composition.borrow().clone();
    match active {
        Some(composition) => {
            let anchor = caret_rect(context, ec, &composition).unwrap_or_else(mouse_anchor);
            window.show(anchor);
        }
        None => window.hide(),
    }
}

/// 组句范围在屏幕上的矩形（`GetActiveView` + `GetTextExt`）。拿不到 / 空矩形返回 `None`。
fn caret_rect(context: &ITfContext, ec: u32, composition: &ITfComposition) -> Option<RECT> {
    // SAFETY: ec 是本会话读写锁；composition 是活动组句。
    unsafe {
        let range = composition.GetRange().ok()?;
        let view = context.GetActiveView().ok()?;
        let mut rect = RECT::default();
        let mut clipped = BOOL(0);
        view.GetTextExt(ec, &range, &mut rect, &mut clipped).ok()?;
        // 有些应用给不出（返回全零 / 空矩形），当作拿不到。
        if rect.right <= rect.left && rect.bottom <= rect.top {
            return None;
        }
        Some(rect)
    }
}

/// 应用给不出光标位置时的兜底锚点：鼠标处一个零宽、约一行高的矩形。
fn mouse_anchor() -> RECT {
    let mut point = POINT::default();
    // SAFETY: point 可写。
    let _ = unsafe { GetCursorPos(&mut point) };
    RECT {
        left: point.x,
        top: point.y,
        right: point.x,
        bottom: point.y + 16,
    }
}

/// 落定上屏文本：组句进行中就把组句范围替换成它再结束组句（文本留在文档里成为普通文本），否则在选区插入。
fn commit_text(shared: &Rc<Shared>, context: &ITfContext, ec: u32, text: &str) -> Result<()> {
    let utf16: Vec<u16> = text.encode_utf16().collect();
    let active = shared.composition.borrow().clone();
    match active {
        Some(composition) => {
            // SAFETY: ec 是本会话写锁；composition 是我们起的活动组句。
            let range = unsafe { composition.GetRange()? };
            unsafe { range.SetText(ec, 0, &utf16)? };
            move_selection_to_end(context, ec, &range)?;
            unsafe { composition.EndComposition(ec)? };
            *shared.composition.borrow_mut() = None;
        }
        None => {
            let insert: ITfInsertAtSelection = context.cast()?;
            // SAFETY: ec 有效；标志 0 正常插入并回传范围（不用 NOQUERY，见 edit_session 注释）。
            let _range = unsafe {
                insert.InsertTextAtSelection(ec, INSERT_TEXT_AT_SELECTION_FLAGS(0), &utf16)?
            };
        }
    }
    Ok(())
}

/// 把组句拼音行更新成 `preedit`：没有活动组句就在选区处起一个，再整段替换文本并把光标移到末尾。
fn update_preedit(shared: &Rc<Shared>, context: &ITfContext, ec: u32, preedit: &str) -> Result<()> {
    // 先把当前句柄取到本地再 match：不能直接 `match shared.composition.borrow().clone()`——那样
    // `borrow()` 的读借用会一直挂到整个 match 体结束，`None` 分支里 `start_composition` 再 `borrow_mut`
    // 就会 BorrowMutError panic（首字母必炸）。
    let active = shared.composition.borrow().clone();
    let composition = match active {
        Some(composition) => composition,
        None => start_composition(shared, context, ec)?,
    };
    let utf16: Vec<u16> = preedit.encode_utf16().collect();
    // SAFETY: ec 是写锁；composition 活动中。
    let range = unsafe { composition.GetRange()? };
    unsafe { range.SetText(ec, 0, &utf16)? };
    move_selection_to_end(context, ec, &range)?;
    Ok(())
}

/// 在当前选区处起一个空组句并存起来。组句 sink（[`CompositionSink`]）随组句一起交给框架持有。
fn start_composition(shared: &Rc<Shared>, context: &ITfContext, ec: u32) -> Result<ITfComposition> {
    let insert: ITfInsertAtSelection = context.cast()?;
    // QUERYONLY：不插入，只取选区处的空范围当组句起点。
    // SAFETY: ec 有效；空文本切片。
    let range = unsafe { insert.InsertTextAtSelection(ec, TF_IAS_QUERYONLY, &[])? };
    let context_composition: ITfContextComposition = context.cast()?;
    let sink: ITfCompositionSink = CompositionSink::new(shared.clone()).into();
    // SAFETY: ec 是写锁；range 是刚取到的选区范围；sink 是本模块造的、框架会 AddRef 持有。
    let composition = unsafe { context_composition.StartComposition(ec, &range, &sink)? };
    *shared.composition.borrow_mut() = Some(composition.clone());
    Ok(composition)
}

/// 收掉组句（若有）：清空组句文本再结束，避免残留拼音。
fn end_composition(shared: &Rc<Shared>, ec: u32) -> Result<()> {
    if let Some(composition) = shared.composition.borrow_mut().take() {
        // SAFETY: ec 是写锁；composition 是我们起的活动组句。
        let range = unsafe { composition.GetRange()? };
        unsafe { range.SetText(ec, 0, &[])? };
        unsafe { composition.EndComposition(ec)? };
    }
    Ok(())
}

/// 把选区折叠到 `range` 的末尾（组句更新后光标停在拼音行最后）。
fn move_selection_to_end(context: &ITfContext, ec: u32, range: &ITfRange) -> Result<()> {
    // SAFETY: ec 是写锁；range 属于本上下文。
    let end = unsafe { range.Clone()? };
    unsafe { end.Collapse(ec, TF_ANCHOR_END)? };
    let selection = TF_SELECTION {
        range: ManuallyDrop::new(Some(end)),
        style: TF_SELECTIONSTYLE {
            ase: TF_AE_END,
            fInterimChar: false.into(),
        },
    };
    // SAFETY: ec 是写锁；SetSelection 只读这个数组、不接管所有权。
    let result = unsafe { context.SetSelection(ec, std::slice::from_ref(&selection)) };
    // SetSelection 不接管 range，这里手动把 ManuallyDrop 里的接口释放掉，避免泄漏。
    drop(ManuallyDrop::into_inner(selection.range));
    result
}

/// 组句终止回调对象：应用强行结束我们的组句（如点到别处）时，框架调 [`OnCompositionTerminated`]，
/// 我们借它清掉本地组句状态。
///
/// [`OnCompositionTerminated`]: ITfCompositionSink_Impl::OnCompositionTerminated
#[implement(ITfCompositionSink)]
pub(crate) struct CompositionSink {
    shared: Rc<Shared>,
}

impl CompositionSink {
    fn new(shared: Rc<Shared>) -> Self {
        Self { shared }
    }
}

impl ITfCompositionSink_Impl for CompositionSink_Impl {
    fn OnCompositionTerminated(
        &self,
        _ecwrite: u32,
        _composition: Ref<ITfComposition>,
    ) -> Result<()> {
        super::log::log("组句被应用终止，清本地状态");
        self.shared.reset();
        Ok(())
    }
}
