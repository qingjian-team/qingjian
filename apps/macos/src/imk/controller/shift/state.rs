//! 每个 IMK 控制器的修饰键手势，不随全局英文状态跨焦点保存。

use super::tap::ShiftTap;
use std::cell::{Cell, RefCell};

#[derive(Default)]
pub struct ControllerState {
    pub(in crate::imk::controller) shift: RefCell<ShiftTap>,

    pub(in crate::imk::controller) generation: Cell<u64>,
}
