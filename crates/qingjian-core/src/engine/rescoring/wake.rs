//! 后台模型交了结果就叫醒壳：壳的工人循环原本隔 20 毫秒轮询一次，结果算完要等到下一拍才看得见，
//! 知意出字、打分两轮各白等十几毫秒。壳在 [`crate::Engine::set_rescoring_wake`] 里给一个往自己消息队列投信的回调。

use std::sync::{Arc, Mutex};

/// 结果到了时调一下；在后台线程上调，只该投个信、不该做重活。
pub type Wake = Arc<dyn Fn() + Send + Sync>;

/// 引擎与后台线程共用的那个回调：模型可能先于回调接上，换回调时线程里看到的也跟着换。
#[derive(Clone, Default)]
pub(crate) struct WakeSlot(Arc<Mutex<Option<Wake>>>);

impl WakeSlot {
    pub fn set(&self, wake: Option<Wake>) {
        if let Ok(mut slot) = self.0.lock() {
            *slot = wake;
        }
    }

    /// 结果已经放进通道之后调。
    pub fn ring(&self) {
        let wake = self.0.lock().ok().and_then(|slot| slot.clone());
        if let Some(wake) = wake {
            wake();
        }
    }
}
