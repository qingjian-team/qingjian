//! 一段组句的键数与打字时长，上屏时连同用量一起交给统计。

use std::time::{Duration, Instant};

/// 键与键之间停顿超过这么久就当没在打字（想词、走开），那段不算进打字时长。
const PAUSE: Duration = Duration::from_secs(5);

/// 从上一次上屏起数的键数、打字时长与有没有删过：词与词之间的间隔也算打字（停顿不太久的话），部分上屏后接着数下一段。
#[derive(Debug, Clone, Default)]
pub struct Typing {
    keys: u64,

    active: Duration,

    /// 上一键（或上一次上屏）的时刻。
    last: Option<Instant>,

    edited: bool,
}

impl Typing {
    pub fn key(&mut self, now: Instant) {
        self.tick(now);
        self.keys += 1;
    }

    pub fn edit(&mut self, now: Instant) {
        self.tick(now);
        self.edited = true;
    }

    /// 上屏：交出键数、打字毫秒与删过没有，从这一刻起重新数。
    pub fn take(&mut self, now: Instant) -> (u64, u64, bool) {
        self.tick(now);
        let taken = (self.keys, self.active.as_millis() as u64, self.edited);
        self.keys = 0;
        self.active = Duration::ZERO;
        self.edited = false;
        taken
    }

    /// 新起一段组句、或这段放弃了：键数与删过清零，时间接着上一键算。
    pub fn restart(&mut self) {
        self.keys = 0;
        self.active = Duration::ZERO;
        self.edited = false;
    }

    /// 换了输入上下文（私密输入）：什么都不留。
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    fn tick(&mut self, now: Instant) {
        if let Some(last) = self.last {
            let gap = now.saturating_duration_since(last);
            if gap <= PAUSE {
                self.active += gap;
            }
        }
        self.last = Some(now);
    }
}

#[cfg(test)]
mod tests {
    use super::Typing;
    use std::time::{Duration, Instant};

    #[test]
    fn long_pauses_do_not_count_as_typing() {
        let start = Instant::now();
        let at = |ms| start + Duration::from_millis(ms);
        let mut typing = Typing::default();
        typing.key(at(0));
        typing.key(at(200));
        typing.edit(at(400));
        // 停了 10 秒再选词
        typing.key(at(10_400));
        assert_eq!(typing.take(at(10_600)), (3, 600, true));
        // 部分上屏后接着打：从上屏那一刻起算
        typing.key(at(10_900));
        assert_eq!(typing.take(at(11_000)), (1, 400, false));
        // 新起一段：上屏到第一键的间隔也算，放弃的那段不留键数
        typing.restart();
        typing.key(at(12_000));
        typing.restart();
        typing.key(at(12_500));
        assert_eq!(typing.take(at(12_600)), (1, 600, false));
        typing.reset();
        assert_eq!(typing.take(at(20_000)), (0, 0, false));
    }
}
