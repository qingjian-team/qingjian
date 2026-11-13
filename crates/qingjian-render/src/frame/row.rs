//! 候选窗口的一行：序号、候选词、annotation 片段。

use std::ops::Range;

use super::tone::Tone;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// 显示用序号文本，如 `1`。
    pub index: String,

    /// 候选词。
    pub text: String,

    /// 候选词里和首选不一样的字（`text` 的字节范围，Core 的 `differs_from_lead` 算的），主题写了 `diff` 颜色时画成那个颜色。
    pub diff: Vec<Range<usize>>,

    /// 紧跟在候选词后面的辅码，如 `[kf]`：码是词本身的属性，不进右侧的 annotation 列。
    pub code: Option<String>,

    /// 右侧 annotation，按顺序绘制；没有译文时为空。
    pub annotation: Vec<(String, Tone)>,

    /// 来自云联想：词前画一个小云朵，与本地候选区分。
    pub cloud: bool,
}

impl Row {
    /// 只有序号和候选词的一行。
    pub fn plain(index: usize, text: impl Into<String>) -> Self {
        Self {
            index: (index + 1).to_string(),
            text: text.into(),
            diff: Vec::new(),
            code: None,
            annotation: Vec::new(),
            cloud: false,
        }
    }

    /// 换成显示用的文字（截断、换行画成符号）：差异范围只留两份文字相同的开头里的，截掉的部分不再标。
    pub fn show_text(&mut self, text: String) {
        let same = self
            .text
            .bytes()
            .zip(text.bytes())
            .take_while(|(a, b)| a == b)
            .count();
        self.diff
            .retain(|range| range.end <= same && text.is_char_boundary(range.end));
        self.text = text;
    }
}

#[cfg(test)]
mod tests {
    use super::Row;

    #[test]
    #[allow(clippy::single_range_in_vec_init)]
    fn clipping_drops_the_ranges_it_cuts() {
        let mut row = Row::plain(0, "事先一边输入中文整句");
        row.diff = vec![0..6, 24..30];
        row.show_text("事先一边输入…".to_owned());
        assert_eq!(row.diff, [0..6]);
    }
}
