//! 实时翻译条的显示状态；更新期间保留最近一次成功译文。

#[derive(Default)]
pub(super) struct DisplayState {
    successful: Option<String>,

    pending: bool,

    error: Option<String>,
}

impl DisplayState {
    pub fn begin(&mut self, unavailable: Option<&str>) {
        self.pending = unavailable.is_none();
        self.error = unavailable.map(str::to_owned);
    }

    pub fn complete(&mut self, result: Result<String, String>) {
        self.pending = false;
        match result {
            Ok(text) => {
                self.successful = Some(text);
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn presentation(&self) -> (&str, &str) {
        match (&self.successful, self.pending, &self.error) {
            (Some(text), true, _) => (text, "本地实时翻译 · 更新中…"),
            (Some(text), false, Some(_)) => (text, "本地实时翻译 · 更新失败，显示上次译文"),
            (Some(text), false, None) => (text, "本地实时翻译 · 预译会随输入修正"),
            (None, _, Some(error)) => (error, "本地翻译暂不可用"),
            (None, _, None) => ("翻译中…", "本地实时翻译"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DisplayState;

    #[test]
    fn first_request_waits_then_success_remains_visible_while_updating() {
        let mut display = DisplayState::default();
        display.begin(None);
        assert_eq!(display.presentation().0, "翻译中…");
        display.complete(Ok("I am learning.".to_owned()));
        display.begin(None);
        assert_eq!(
            display.presentation(),
            ("I am learning.", "本地实时翻译 · 更新中…")
        );
        display.complete(Ok("I am learning English.".to_owned()));
        assert_eq!(display.presentation().0, "I am learning English.");
        assert!(!display.presentation().1.contains("更新中"));
    }

    #[test]
    fn failed_update_keeps_success_and_retry_does_not_keep_old_error() {
        let mut display = DisplayState::default();
        display.complete(Ok("previous".to_owned()));
        display.begin(None);
        display.complete(Err("请求超时".to_owned()));
        assert_eq!(display.presentation().0, "previous");
        assert!(display.presentation().1.contains("更新失败"));
        display.begin(None);
        assert_eq!(
            display.presentation(),
            ("previous", "本地实时翻译 · 更新中…")
        );
        display.reset();
        assert_eq!(display.presentation().0, "翻译中…");
    }

    #[test]
    fn first_failure_and_unavailable_configuration_are_visible() {
        let mut display = DisplayState::default();
        display.begin(None);
        display.complete(Err("无法连接服务".to_owned()));
        assert_eq!(display.presentation().0, "无法连接服务");
        display.begin(None);
        assert_eq!(display.presentation().0, "翻译中…");
        display.begin(Some("地址无效"));
        assert_eq!(display.presentation().0, "地址无效");
    }
}
