//! 中英文切换状态：组合键与 Caps Lock 都翻转模式，不修改硬件锁定状态。

#[derive(Default)]
pub(super) struct InputMode {
    /// 与 Caps Lock 状态异或；跨输入框保留，只在输入法进程退出时重置。
    inverted: bool,
}

impl InputMode {
    /// 实际模式，菜单栏与按键处理必须使用同一结果。
    pub fn english(&self, caps_lock: bool) -> bool {
        caps_lock ^ self.inverted
    }

    /// 处理一次非重复的切换按键；调用前由控制器结束当前组句。
    pub fn toggle(&mut self) {
        self.inverted = !self.inverted;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_and_caps_lock_each_toggle_the_current_mode() {
        let mut mode = InputMode::default();
        assert!(!mode.english(false));
        assert!(mode.english(true));
        mode.toggle();
        assert!(mode.english(false));
        assert!(!mode.english(true));
        mode.toggle();
        assert!(!mode.english(false));
        assert!(mode.english(true));
    }
}
