//! 从别的线程把一段活交给主线程做并等它做完：AppKit 的窗口只能在主线程碰。
//! 主线程跑着窗口宿主的事件循环，GCD 的主队列由它顺带清空。

use std::ffi::c_void;

use objc2::MainThreadMarker;

#[repr(C)]
struct DispatchQueue {
    _opaque: [u8; 0],
}

unsafe extern "C" {
    /// `dispatch_get_main_queue()` 是取它地址的宏。
    static _dispatch_main_q: DispatchQueue;

    fn dispatch_sync_f(
        queue: *const DispatchQueue,
        context: *mut c_void,
        work: extern "C" fn(*mut c_void),
    );
}

/// 在主线程上跑 `work`，等它跑完拿回结果。已经在主线程就直接跑。
/// 主线程不能反过来等调用方，否则两边互等；霓虹灯线程只等主线程，主线程从不等它。
pub fn run_on_main<R: Send, F: FnOnce(MainThreadMarker) -> R + Send>(work: F) -> R {
    if let Some(mtm) = MainThreadMarker::new() {
        return work(mtm);
    }

    extern "C" fn trampoline<R, F: FnOnce(MainThreadMarker) -> R>(context: *mut c_void) {
        // SAFETY: context 指向下面栈上的 slot，dispatch_sync_f 返回前一直有效、只有这里用它。
        let slot = unsafe { &mut *context.cast::<(Option<F>, Option<R>)>() };
        // SAFETY: 主队列上的活一定在主线程跑。
        let mtm = unsafe { MainThreadMarker::new_unchecked() };
        if let Some(work) = slot.0.take() {
            slot.1 = Some(work(mtm));
        }
    }

    let mut slot: (Option<F>, Option<R>) = (Some(work), None);
    // SAFETY: 主队列是进程级的静态对象；同步派发，返回时 trampoline 已经跑完。
    unsafe {
        dispatch_sync_f(
            &raw const _dispatch_main_q,
            (&raw mut slot).cast(),
            trampoline::<R, F>,
        );
    }
    slot.1.expect("main queue ran the work")
}
