use std::rc::Rc;

use windows::Win32::UI::TextServices::{
    ITfComposition, ITfCompositionSink, ITfCompositionSink_Impl,
};
use windows::core::{Ref, Result, implement};

use super::Shared;
use crate::com::log::log;

/// 应用强行结束我们的组句（如点到别处）时框架回调它，借此清掉本地组句状态。
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
        self.shared.reset();
        Ok(())
    }
}
