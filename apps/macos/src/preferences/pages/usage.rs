//! 「统计」页：用青简打了多少字。今天 / 最近 7 天 / 累计 三行，汉字 / 中文词 / 英文词 / 上屏次数 四列，
//! 再把累计汉字数折成「几本《某书》」给个直观参照；下面一块是学习语言的词汇（见过 / 看熟 / 上屏过 / 打出过的译词数）。
//! 数据来自 `Engine::usage_summary` / `Engine::vocabulary_summary`，打开窗口时更新。

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSFont, NSTextAlignment, NSTextField};
use objc2_foundation::NSString;
use qingjian_core::{FRESH_UNTIL, Usage, UsageSummary, VocabularySummary, book_scale};

use crate::preferences::controls::{GROUP_GAP, caption, note_full, small_label};
use crate::preferences::layout::{LABEL_WIDTH, Layout, PAGE_PADDING, ROW_HEIGHT};

/// 三行的标题，顺序与 [`UsagePage::show`] 里取值一致。
const ROWS: [&str; 3] = ["今天", "最近 7 天", "累计"];

/// 四列的标题。
const COLUMNS: [&str; 4] = ["汉字", "中文词", "英文词", "上屏次数"];

/// 等级块的四列。
const LEVEL_COLUMNS: [&str; 4] = ["词表", "见过", "看熟", "上屏过"];

/// 等级块最多几行（CEFR 六级、JLPT 五级）。
const MAX_LEVEL_ROWS: usize = 6;

/// 词汇总览那一行的高度：两行字，数字多的时候折行而不是截断。
const VOCABULARY_LINE_HEIGHT: f64 = ROW_HEIGHT * 1.8;

/// 「统计」页里要按数据刷新的控件。
pub struct UsagePage {
    /// 数字格子，按行优先排：`cells[row * 4 + column]`。
    cells: Vec<Retained<NSTextField>>,

    /// 「累计 xx 字，约等于 n 本《某书》」。
    scale: Retained<NSTextField>,

    /// 「自 某日 起，记了 n 天」。
    since: Retained<NSTextField>,

    /// 词汇块标题：「词汇（英语）」。
    vocabulary_title: Retained<NSTextField>,

    /// 词汇块正文：见过 / 看熟 / 上屏过 / 打出过 / 本周新见。
    vocabulary_line: Retained<NSTextField>,

    /// 等级块的表头（没有等级表时整块隐藏）。
    level_headers: Vec<Retained<NSTextField>>,

    /// 等级块每行的等级名。
    level_names: Vec<Retained<NSTextField>>,

    /// 等级块的数字格子，按行优先排：`level_cells[row * 4 + column]`。
    level_cells: Vec<Retained<NSTextField>>,
}

impl UsagePage {
    /// 把「统计」页的控件摆进 `layout`。
    pub fn build(layout: &mut Layout, mtm: MainThreadMarker) -> Self {
        let column_width = (layout.inner_width() - LABEL_WIDTH) / COLUMNS.len() as f64;
        let column_x = |column: usize| PAGE_PADDING + LABEL_WIDTH + column_width * column as f64;
        for (column, title) in COLUMNS.iter().enumerate() {
            let header = small_label(mtm, title);
            header.setAlignment(NSTextAlignment::Right);
            layout.place(&header, column_x(column), column_width, ROW_HEIGHT * 0.7);
        }
        layout.next_row(ROW_HEIGHT * 0.7);
        // SAFETY: NSFontWeightRegular 是 AppKit 导出的常量，只读。
        let weight = unsafe { objc2_app_kit::NSFontWeightRegular };
        let mut cells = Vec::with_capacity(ROWS.len() * COLUMNS.len());
        for title in ROWS {
            let label = caption(mtm, title);
            layout.place(&label, PAGE_PADDING, LABEL_WIDTH, ROW_HEIGHT);
            for column in 0..COLUMNS.len() {
                let cell = NSTextField::labelWithString(&NSString::from_str("0"), mtm);
                cell.setAlignment(NSTextAlignment::Right);
                cell.setFont(Some(&NSFont::monospacedDigitSystemFontOfSize_weight(
                    13.0, weight,
                )));
                layout.place(&cell, column_x(column), column_width, ROW_HEIGHT);
                cells.push(cell);
            }
            layout.next_row(ROW_HEIGHT);
        }
        layout.space(GROUP_GAP);
        let scale = NSTextField::labelWithString(&NSString::from_str(""), mtm);
        scale.setFont(Some(&NSFont::boldSystemFontOfSize(13.0)));
        layout.place(&scale, PAGE_PADDING, layout.inner_width(), ROW_HEIGHT);
        layout.next_row(ROW_HEIGHT);
        let since = small_label(mtm, "");
        layout.place(&since, PAGE_PADDING, layout.inner_width(), ROW_HEIGHT * 0.7);
        layout.next_row(ROW_HEIGHT * 0.7);
        layout.space(GROUP_GAP);
        note_full(
            layout,
            mtm,
            "数的是上屏的文字：选一个词算一个中文词，整句按词切开数；英文候选、回车原样上屏的英文词与英文译词算英文词。只在这台电脑上数，与输入日志无关，关掉或清空日志不影响这里。",
        );
        layout.space(GROUP_GAP);
        let vocabulary_title = NSTextField::labelWithString(&NSString::from_str(""), mtm);
        vocabulary_title.setFont(Some(&NSFont::boldSystemFontOfSize(13.0)));
        layout.place(
            &vocabulary_title,
            PAGE_PADDING,
            layout.inner_width(),
            ROW_HEIGHT,
        );
        layout.next_row(ROW_HEIGHT);
        // 数字大了一行放不下（「本周新见 4,63…」被截断过），允许折成两行
        let vocabulary_line = NSTextField::labelWithString(&NSString::from_str(""), mtm);
        vocabulary_line.setUsesSingleLineMode(false);
        if let Some(cell) = vocabulary_line.cell() {
            cell.setWraps(true);
        }
        layout.place(
            &vocabulary_line,
            PAGE_PADDING,
            layout.inner_width(),
            VOCABULARY_LINE_HEIGHT,
        );
        layout.next_row(VOCABULARY_LINE_HEIGHT);
        let mut level_headers = Vec::with_capacity(LEVEL_COLUMNS.len());
        for (column, title) in LEVEL_COLUMNS.iter().enumerate() {
            let header = small_label(mtm, title);
            header.setAlignment(NSTextAlignment::Right);
            layout.place(&header, column_x(column), column_width, ROW_HEIGHT * 0.7);
            level_headers.push(header);
        }
        layout.next_row(ROW_HEIGHT * 0.7);
        let mut level_names = Vec::with_capacity(MAX_LEVEL_ROWS);
        let mut level_cells = Vec::with_capacity(MAX_LEVEL_ROWS * LEVEL_COLUMNS.len());
        for _ in 0..MAX_LEVEL_ROWS {
            let name = caption(mtm, "");
            layout.place(&name, PAGE_PADDING, LABEL_WIDTH, ROW_HEIGHT * 0.8);
            level_names.push(name);
            for column in 0..LEVEL_COLUMNS.len() {
                let cell = NSTextField::labelWithString(&NSString::from_str(""), mtm);
                cell.setAlignment(NSTextAlignment::Right);
                cell.setFont(Some(&NSFont::monospacedDigitSystemFontOfSize_weight(
                    12.0, weight,
                )));
                layout.place(&cell, column_x(column), column_width, ROW_HEIGHT * 0.8);
                level_cells.push(cell);
            }
            layout.next_row(ROW_HEIGHT * 0.8);
        }
        note_full(
            layout,
            mtm,
            &format!(
                "候选右侧橙色的译词是生词：你上屏时它在候选窗口里出现还不到 {FRESH_UNTIL} 次。看熟了就变回灰色。只记译词本身和次数，不记你打了什么。"
            ),
        );
        Self {
            cells,
            scale,
            since,
            vocabulary_title,
            vocabulary_line,
            level_headers,
            level_names,
            level_cells,
        }
    }

    /// 按汇总刷新。`language` 是学习语言的显示名（「英语」）。
    pub fn show(&self, summary: &UsageSummary, vocabulary: &VocabularySummary, language: &str) {
        let rows = [summary.today, summary.week, summary.total];
        for (row, usage) in rows.iter().enumerate() {
            for (column, value) in columns(usage).into_iter().enumerate() {
                self.cells[row * COLUMNS.len() + column]
                    .setStringValue(&NSString::from_str(&group_digits(value)));
            }
        }
        self.scale
            .setStringValue(&NSString::from_str(&scale_line(summary.total.hanzi)));
        let since = match &summary.since {
            Some(date) => format!("自 {date} 起，有输入的天数 {}。", summary.days),
            None => "还没有记录，打几个字再来看。".to_owned(),
        };
        self.since.setStringValue(&NSString::from_str(&since));
        self.vocabulary_title
            .setStringValue(&NSString::from_str(&format!("词汇（{language}）")));
        self.vocabulary_line
            .setStringValue(&NSString::from_str(&vocabulary_line(vocabulary)));
        let levels = &vocabulary.levels;
        for header in &self.level_headers {
            header.setHidden(levels.is_empty());
        }
        for (row, name) in self.level_names.iter().enumerate() {
            let level = levels.get(row);
            name.setHidden(level.is_none());
            name.setStringValue(&NSString::from_str(
                level.map_or("", |level| level.name.as_str()),
            ));
            let values = level.map_or([0; 4], |level| {
                [level.total, level.seen, level.familiar, level.committed]
            });
            for (column, value) in values.into_iter().enumerate() {
                let cell = &self.level_cells[row * LEVEL_COLUMNS.len() + column];
                cell.setHidden(level.is_none());
                cell.setStringValue(&NSString::from_str(&group_digits(value)));
            }
        }
    }
}

/// 「见过 120 个词，看熟 80 个；上屏过 60 个，直接打出过 3 个；本周新见 12 个。」
fn vocabulary_line(summary: &VocabularySummary) -> String {
    if summary.seen == 0 {
        return "还没见过译词：打中文时候选右侧的译词就是词汇的来源。".to_owned();
    }
    format!(
        "见过 {} 个词，看熟 {} 个；上屏过 {} 个，直接打出过 {} 个；本周新见 {} 个。",
        group_digits(summary.seen),
        group_digits(summary.familiar),
        group_digits(summary.committed),
        group_digits(summary.used),
        group_digits(summary.new_this_week)
    )
}

/// 一行四列的值，顺序同 [`COLUMNS`]。
fn columns(usage: &Usage) -> [u64; 4] {
    [usage.hanzi, usage.words, usage.english_words, usage.commits]
}

/// 「累计输入 12.3 万字，约等于 1.7 本《活着》（约 12 万字）。」；一个字没有时不比。
fn scale_line(hanzi: u64) -> String {
    if hanzi == 0 {
        return "累计输入 0 字。".to_owned();
    }
    let (book, ratio) = book_scale(hanzi);
    format!(
        "累计输入 {}，约等于 {} 本《{}》（约 {}）。",
        hanzi_text(hanzi),
        format_ratio(ratio),
        book.title,
        hanzi_text(book.hanzi)
    )
}

/// 千位分隔：`12345` → `12,345`。
fn group_digits(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// 中文习惯的字数写法：万以下写整数（`2,500 字`），万以上一位小数（`12.3 万字`），亿以上同理。
fn hanzi_text(value: u64) -> String {
    const WAN: f64 = 10_000.0;
    const YI: f64 = 100_000_000.0;
    let value_f = value as f64;
    if value_f >= YI {
        format!("{} 亿字", trim_decimal(value_f / YI))
    } else if value_f >= WAN {
        format!("{} 万字", trim_decimal(value_f / WAN))
    } else {
        format!("{} 字", group_digits(value))
    }
}

/// 一位小数，`.0` 去掉。
fn trim_decimal(value: f64) -> String {
    let text = format!("{value:.1}");
    text.strip_suffix(".0").map_or(text.clone(), str::to_owned)
}

/// 倍数：不到 10 倍留一位小数，再多就取整。
fn format_ratio(ratio: f64) -> String {
    if ratio >= 10.0 {
        format!("{}", ratio.round() as u64)
    } else {
        trim_decimal(ratio)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_digits_and_scales_in_chinese_units() {
        assert_eq!(group_digits(0), "0");
        assert_eq!(group_digits(999), "999");
        assert_eq!(group_digits(1000), "1,000");
        assert_eq!(group_digits(1_234_567), "1,234,567");
        assert_eq!(hanzi_text(9_999), "9,999 字");
        assert_eq!(hanzi_text(10_000), "1 万字");
        assert_eq!(hanzi_text(123_456), "12.3 万字");
        assert_eq!(hanzi_text(250_000_000), "2.5 亿字");
    }

    #[test]
    fn describes_the_total_against_a_book() {
        assert_eq!(scale_line(0), "累计输入 0 字。");
        assert_eq!(
            scale_line(2_500),
            "累计输入 2,500 字，约等于 0.5 本《道德经》（约 5,000 字）。"
        );
        assert_eq!(
            scale_line(204_000),
            "累计输入 20.4 万字，约等于 1.7 本《活着》（约 12 万字）。"
        );
        assert_eq!(
            scale_line(12_000_000),
            "累计输入 1200 万字，约等于 12 本《平凡的世界》（约 100 万字）。"
        );
    }
}
