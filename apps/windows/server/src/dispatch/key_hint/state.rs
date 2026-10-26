//! 「卡住时显示」的状态：这一段组句里有没有弹出过、上次敲到哪、停顿到什么时候算卡住。
//! Server 没有定时器，到点靠工人循环按 [`KeyHintState::next_deadline`] 来一次 `tick`。

use std::time::{Duration, Instant};

/// 停多久算卡住。熟练的人两键之间远短于这个，平时看不到键盘。与 macOS 一致。
const HESITATION: Duration = Duration::from_millis(700);

#[derive(Debug, Default)]
pub struct KeyHintState {
    /// 上次看到的缓冲区，变了就是又敲了一键，停顿重新计时。
    typed: String,

    /// 这一段组句里已经弹出过：之后一直留着到组句结束，不随每键闪。
    shown: bool,

    /// 停到这个时刻还没再敲就弹出。
    deadline: Option<Instant>,
}

impl KeyHintState {
    /// 这一帧要不要画键盘。`stuck` 是敲出了拼不成音节的键，当场弹出；否则缓冲区变了就重新计时。
    pub fn update(&mut self, typed: &str, stuck: bool) -> bool {
        if stuck || self.shown {
            self.shown = true;
            self.deadline = None;
            return true;
        }
        if typed != self.typed {
            typed.clone_into(&mut self.typed);
            self.deadline = Some(Instant::now() + HESITATION);
        }
        false
    }

    /// 停顿到点了：记成弹出过，返回 `true`；没到点或没在计时返回 `false`。
    pub fn hesitated(&mut self) -> bool {
        if self
            .deadline
            .is_none_or(|deadline| Instant::now() < deadline)
        {
            return false;
        }
        self.shown = true;
        self.deadline = None;
        true
    }

    /// 离停顿到点还有多久；没在计时为 `None`。
    pub fn next_deadline(&self) -> Option<Duration> {
        self.deadline
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
    }
}
