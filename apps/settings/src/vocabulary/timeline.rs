//! 时间线的数：某天的相遇、当天第一次见到的生词、连续输入几天、按小时的色带，以及前后有记录的日子。

use jiff::civil::Date;
use qingjian_learning::{LoggedEncounter, VocabularyEntry};

use super::Vocabulary;

impl Vocabulary {
    /// 那天的相遇，新的在前。
    pub fn day_encounters(&self) -> Vec<&LoggedEncounter> {
        self.encounters
            .iter()
            .rev()
            .filter(|encounter| encounter.time.date() == self.day)
            .collect()
    }

    /// 那天第一次见到的译词个数。
    pub fn first_seen(&self) -> usize {
        self.book
            .words(self.language)
            .filter(|(_, entry)| entry.first == self.day)
            .count()
    }

    /// 到那天为止连着打字的天数；那天没打就从前一天往回数。
    pub fn streak(&self) -> usize {
        let mut day = self.day;
        if !self.typed_days.contains(&day) {
            day = day.yesterday().unwrap_or(day);
        }
        let mut count = 0;
        while self.typed_days.contains(&day) {
            count += 1;
            match day.yesterday() {
                Ok(previous) => day = previous,
                Err(_) => break,
            }
        }
        count
    }

    /// 那天每个钟点的相遇次数。
    pub fn hours(&self) -> [usize; 24] {
        let mut hours = [0; 24];
        for encounter in self.day_encounters() {
            hours[encounter.time.hour() as usize] += 1;
        }
        hours
    }

    /// 前一个、后一个有相遇的日子（后一个最多到今天）。
    pub fn neighbors(&self) -> (Option<Date>, Option<Date>) {
        let previous = self
            .encounters
            .iter()
            .rev()
            .map(|encounter| encounter.time.date())
            .find(|date| *date < self.day);
        let next = self
            .encounters
            .iter()
            .map(|encounter| encounter.time.date())
            .find(|date| *date > self.day)
            .or((self.day < self.today).then_some(self.today));
        (previous, next)
    }

    pub fn is_today(&self) -> bool {
        self.day == self.today
    }

    /// 这条相遇的说法在词表里的记录（单个词才有，整句没有），拿例句与生词标记。
    pub fn entry_of(&self, encounter: &LoggedEncounter) -> Option<&VocabularyEntry> {
        self.book.get(self.language, &encounter.translation)
    }
}
