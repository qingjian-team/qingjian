//! 配置文件监视：输入法激活期间每秒看一眼 `config.toml` 的修改时间，变了就热加载。
//! 一次 `stat` 的开销可以忽略，不值得为此引 FSEvents。同一个定时器顺带定期把学习数据落盘（`Host::tick`）。

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_foundation::{NSObject, NSObjectProtocol, NSTimer};

/// 检查间隔（秒）。
const POLL_INTERVAL: f64 = 1.0;

pub struct ConfigWatch {
    /// 定时器；未激活时为 `None`。
    timer: Option<Retained<NSTimer>>,

    mtm: MainThreadMarker,
}

impl ConfigWatch {
    pub fn new(mtm: MainThreadMarker) -> Self {
        Self { timer: None, mtm }
    }

    pub fn start(&mut self) {
        if self.timer.is_some() {
            return;
        }
        let target = ConfigWatchTick::new(self.mtm);
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                POLL_INTERVAL,
                &target,
                sel!(tick:),
                None,
                true,
            )
        };
        self.timer = Some(timer);
    }

    pub fn stop(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.invalidate();
        }
    }
}

define_class!(
    // SAFETY: NSObject 没有子类化要求；没有实现 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    struct ConfigWatchTick;

    impl ConfigWatchTick {
        #[unsafe(method(tick:))]
        fn tick(&self, _timer: Option<&AnyObject>) {
            // 定时器回调也是 ObjC 运行时直接调的，panic 同样不能穿出去
            if crate::imk::catch_panic("定时器", || crate::host::with(|h| h.tick())).is_none() {
                crate::imk::recover_from_panic(None);
            }
        }
    }

    unsafe impl NSObjectProtocol for ConfigWatchTick {}
);

impl ConfigWatchTick {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
