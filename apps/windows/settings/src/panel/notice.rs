//! 各页操作后的临时提示，挂在 [`Settings`](super::Settings) 上：一行灰字（导入统计等）与一行红字（失败原因）。

/// 上一次操作的提示文案；切页时清掉。
#[derive(Default)]
pub(crate) struct Notice {
    /// 结果说明（导入统计、导入成功等），页面底部灰字显示。
    pub(crate) note: Option<String>,

    /// 失败原因，页面底部红字显示。
    pub(crate) error: Option<String>,
}

impl Notice {
    /// 记下一次成功：写说明、清掉上一次的错误。
    pub(crate) fn succeed(&mut self, note: String) {
        self.note = Some(note);
        self.error = None;
    }

    /// 记下一次失败：写错误、清掉上一次的说明。
    pub(crate) fn fail(&mut self, error: String) {
        self.error = Some(error);
        self.note = None;
    }

    /// 清空两行（换页、成功的设置改动都调）。
    pub(crate) fn clear(&mut self) {
        self.note = None;
        self.error = None;
    }
}
