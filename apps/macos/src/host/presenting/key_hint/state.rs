//! 「卡住时显示」的状态：这一段组句里有没有弹出过、上次敲到哪、停顿计时。

use objc2::MainThreadMarker;

use super::timer::HesitationTimer;

#[derive(Default)]
pub struct KeyHintState {
    /// 上次看到的缓冲区，变了就是又敲了一键，停顿重新计时。
    typed: String,

    /// 这一段组句里已经弹出过：之后一直留着到组句结束，不随每键闪。
    shown: bool,

    timer: Option<HesitationTimer>,
}

impl KeyHintState {
    /// 这一帧要不要画键盘。`stuck` 是敲出了拼不成音节的键，当场弹出；否则缓冲区变了就重新计时，等 [`Self::hesitated`]。
    pub fn update(&mut self, mtm: MainThreadMarker, typed: &str, stuck: bool) -> bool {
        if stuck || self.shown {
            self.shown = true;
            self.timer = None;
            return true;
        }
        if typed != self.typed {
            typed.clone_into(&mut self.typed);
            self.timer = Some(HesitationTimer::schedule(mtm));
        }
        false
    }

    /// 停顿计时到点。
    pub fn hesitated(&mut self) {
        self.shown = true;
        self.timer = None;
    }
}
