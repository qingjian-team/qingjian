//! 「统计」页：输入量（今天 / 7 天 / 累计）、折成几本书、学习语言的词汇与等级分布。
//! 直读 `%APPDATA%\Qingjian` 下的 `usage.tsv` / `user-vocab.tsv`，不经 Server；打开这页时读一次。

use jiff::Zoned;
use qingjian_core::{Language, Usage, UsageSummary, VocabularySummary, book_scale};
use qingjian_learning::{UsageStats, VocabularyBook};
use qingjian_translate::LevelTable;
use windows_reactor::*;

use crate::panel::Settings;
use crate::panel::controls::{note, page, repo_resource};

const COLUMNS: [&str; 4] = ["汉字", "中文词", "英文词", "上屏次数"];

fn language_name(language: Language) -> &'static str {
    match language {
        Language::English => "英语",
        Language::Japanese => "日语",
        Language::Chinese => "中文",
    }
}

fn cell(text: impl Into<String>, width: f64) -> View {
    TextBlock::new().text(text).width(width).into()
}

/// 行首标签 + 四个数字。
fn table_row(label: &str, values: [String; 4], strong: bool) -> View {
    let label_block = TextBlock::new()
        .text(label)
        .width(110.0)
        .font_weight(if strong {
            FontWeight::SEMI_BOLD
        } else {
            FontWeight::NORMAL
        });
    let cells = [
        label_block.into(),
        cell(values[0].clone(), 90.0),
        cell(values[1].clone(), 90.0),
        cell(values[2].clone(), 90.0),
        cell(values[3].clone(), 90.0),
    ];
    StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(8.0)
        .children(cells)
}

fn columns(usage: &Usage) -> [String; 4] {
    [
        group_digits(usage.hanzi),
        group_digits(usage.words),
        group_digits(usage.english_words),
        group_digits(usage.commits),
    ]
}

pub(crate) fn view(settings: &Settings, _context: &mut ViewContext<Settings>) -> View {
    let today = Zoned::now().date();
    let language = settings
        .config
        .general
        .learning_language
        .parse::<Language>()
        .unwrap_or(Language::English);
    let dir = settings.data_dir();

    let usage = UsageStats::open(dir.join("usage.tsv")).summary_on(today);
    let mut book = VocabularyBook::open(dir.join("user-vocab.tsv"));
    if let Some(path) = repo_resource(&format!("assets/levels/levels-{}.tsv", language.code()))
        && let Ok(table) = LevelTable::from_path(path)
    {
        book = book.with_levels(language, table);
    }
    let vocabulary = book.summary_on(language, today);

    let header = table_row("", COLUMNS.map(str::to_owned), true);
    let rows = [
        header,
        table_row("今天", columns(&usage.today), false),
        table_row("最近 7 天", columns(&usage.week), false),
        table_row("累计", columns(&usage.total), false),
    ];

    let body = StackPanel::new().spacing(12.0).children([
        StackPanel::new().spacing(6.0).children(rows),
        TextBlock::new()
            .text(scale_line(usage.total.hanzi))
            .font_weight(FontWeight::SEMI_BOLD)
            .into(),
        note(&since_line(&usage)),
        note("数的是上屏的文字：选一个词算一个中文词，整句按词切开数；英文候选、回车原样上屏的英文词与英文译词算英文词。只在这台电脑上数，与输入日志无关。"),
        TextBlock::new()
            .text(format!("词汇（{}）", language_name(language)))
            .font_weight(FontWeight::SEMI_BOLD)
            .into(),
        note(&vocabulary_line(&vocabulary)),
        level_block(&vocabulary),
    ]);
    page("统计", body)
}

/// 有等级表才显示，一级一行。
fn level_block(vocabulary: &VocabularySummary) -> View {
    if vocabulary.levels.is_empty() {
        return note("装了词汇等级表后，这里按 CEFR / JLPT 等级列词汇分布。");
    }
    let mut rows: Vec<View> = Vec::with_capacity(vocabulary.levels.len() + 1);
    rows.push(table_row(
        "等级",
        ["词表", "见过", "看熟", "上屏过"].map(str::to_owned),
        true,
    ));
    for level in &vocabulary.levels {
        rows.push(table_row(
            &level.name,
            [
                group_digits(level.total),
                group_digits(level.seen),
                group_digits(level.familiar),
                group_digits(level.committed),
            ],
            false,
        ));
    }
    StackPanel::new().spacing(6.0).keyed_children(
        rows.into_iter()
            .enumerate()
            .map(|(index, row)| KeyedView::new(index.to_string(), row)),
    )
}

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
        group_digits(summary.new_this_week),
    )
}

fn since_line(summary: &UsageSummary) -> String {
    match &summary.since {
        Some(date) => format!("自 {date} 起，有输入的天数 {}。", summary.days),
        None => "还没有记录，打几个字再来看。".to_owned(),
    }
}

/// 「累计输入 20.4 万字，约等于 1.7 本《活着》（约 12 万字）。」
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
        hanzi_text(book.hanzi),
    )
}

/// 千位分隔。
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

/// 万以下整数，万 / 亿以上一位小数。
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

/// 不到 10 倍留一位小数，再多取整。
fn format_ratio(ratio: f64) -> String {
    if ratio >= 10.0 {
        format!("{}", ratio.round() as u64)
    } else {
        trim_decimal(ratio)
    }
}
