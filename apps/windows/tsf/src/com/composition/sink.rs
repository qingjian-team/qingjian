use std::rc::Rc;

use windows::Win32::UI::TextServices::{
    ITfComposition, ITfCompositionSink, ITfCompositionSink_Impl,
};
use windows::core::{Ref, Result, implement};

use super::Shared;
use crate::com::log::log;

/// 应用强行结束我们的组句（如点到别处）时框架回调。拼音已被框架定成普通文本，
/// 只清本地状态、记下 Server 缓冲要清，绝不能再把 Server 交出的文本插一次。
#[implement(ITfCompositionSink)]
pub(super) struct CompositionSink {
    shared: Rc<Shared>,
}

impl CompositionSink {
    pub(super) fn new(shared: Rc<Shared>) -> Self {
        Self { shared }
    }
}

impl ITfCompositionSink_Impl for CompositionSink_Impl {
    fn OnCompositionTerminated(
        &self,
        _ecwrite: u32,
        _composition: Ref<ITfComposition>,
    ) -> Result<()> {
        log("组句被应用终止，清本地状态");
        self.shared.terminated();
        Ok(())
    }
}
