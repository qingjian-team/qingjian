//! 把剪贴板内容当成 v 模式的参数：用户**手打** `cb` 才读，不猜、不自动补。
//!
//! 为什么不是在参数空着时自动顶上：那样用的人不知道输入法拿了什么（剪贴板里也可能是密码），
//! 这条在 #1111 / #1163 里已经否过两轮。改成「打 `cb`」以后，读剪贴板这个动作是用户自己
//! 敲出来的，和 `vstat;1;2;3` 里敲数字一样明确。
//!
//! 一个关键字管所有子模式：`vstat;cb`、`vcol;cb`、`vbit;cb`、`vtrans;sort;cb`、`vbase;16;cb`
//! 都走这里——判定在 [`substitute`]，各子模式自己的解析一行都不用改。
//!
//! 剪贴板里有分隔符（换行、分号、逗号、制表符）时拆成多个参数：`1\n2\n3` 与 `1;2;3` 等价，
//! 这样从表格、日志里复制一列数就能直接用。拆出来的个数与每条长度都有上限，见 [`Limits`]；
//! 剪贴板内容不进日志。

/// 用户打的字：参数位置上出现它，换成剪贴板内容。
pub const KEYWORD: &str = "cb";

/// 剪贴板内容的取值口径。壳各自实现一份（Windows / macOS / Linux），Core 不碰系统接口。
pub trait Source {
    /// 当前剪贴板里的文本；不是文本、读不到、或已经很大时返回 `None`。
    fn text(&self) -> Option<String>;
}

/// 没有剪贴板可用时的空实现：开着开关也只会得到 `None`，`cb` 原样留着当普通参数。
pub struct NoClipboard;

impl Source for NoClipboard {
    fn text(&self) -> Option<String> {
        None
    }
}

/// 取值上限。剪贴板里常常是整篇文章，全拉进参数会把一次查询拖成几十毫秒。
pub struct Limits;

impl Limits {
    /// 单条参数最多几个字符，超了截断。
    pub const MAX_CHARS: usize = 2048;

    /// 拆出来的参数最多几条，超了丢弃。
    pub const MAX_ARGS: usize = 256;
}

/// 参数里出现 `cb` 时，把整串参数改写成「剪贴板内容铺开」的样子；没有 `cb` 就返回 `None`。
///
/// `args` 是子模式名之后的那一段（`cb`、`cb;16`、`sort;cb`）。判定按参数逐个来，只有整段
/// 等于 [`KEYWORD`]（大小写不敏感）才算，`cba`、`cb1` 这类普通参数不受影响。
pub fn substitute(args: &str, source: &dyn Source) -> Option<String> {
    if !args
        .split([';', '；'])
        .any(|part| part.trim().eq_ignore_ascii_case(KEYWORD))
    {
        return None;
    }
    let text = source.text()?;
    let mut tokens: Vec<String> = Vec::new();
    for part in args.split([';', '；']) {
        let part = part.trim();
        if part.eq_ignore_ascii_case(KEYWORD) {
            tokens.extend(clipboard_tokens(&text));
        } else if !part.is_empty() {
            tokens.push(part.to_owned());
        }
    }
    tokens.truncate(Limits::MAX_ARGS);
    (!tokens.is_empty()).then(|| tokens.join(";"))
}

/// 剪贴板文本 → 参数列表：按换行、分号、逗号、制表符拆，去掉空项与首尾空白，逐条截断。
fn clipboard_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for piece in text.split(['\n', '\r', ';', '；', ',', '，', '\t']) {
        let piece = piece.trim();
        if piece.is_empty() {
            continue;
        }
        let truncated: String = piece.chars().take(Limits::MAX_CHARS).collect();
        tokens.push(truncated);
        if tokens.len() >= Limits::MAX_ARGS {
            break;
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixed(Option<&'static str>);

    impl Source for Fixed {
        fn text(&self) -> Option<String> {
            self.0.map(str::to_owned)
        }
    }

    #[test]
    fn only_a_whole_token_is_the_keyword() {
        let source = Fixed(Some("42"));
        assert_eq!(substitute("cb", &source).as_deref(), Some("42"));
        assert_eq!(substitute("CB", &source).as_deref(), Some("42"));
        assert_eq!(substitute("sort;cb", &source).as_deref(), Some("sort;42"));
        // 长得像但不是同一个词：照旧当普通参数
        assert_eq!(substitute("cba", &source), None);
        assert_eq!(substitute("cb1;2", &source), None);
        // 没有 cb 时不碰剪贴板
        assert_eq!(substitute("1;2;3", &source), None);
    }

    #[test]
    fn clipboard_text_is_split_into_arguments() {
        let source = Fixed(Some("1\n2\n2\n4"));
        assert_eq!(substitute("cb", &source).as_deref(), Some("1;2;2;4"));
        let source = Fixed(Some(" 7 , 8 ；9 "));
        assert_eq!(substitute("cb", &source).as_deref(), Some("7;8;9"));
        // 与手打的参数混着用：cb 展开后前后都在
        let source = Fixed(Some("2;3"));
        assert_eq!(substitute("1;cb;4", &source).as_deref(), Some("1;2;3;4"));
    }

    #[test]
    fn empty_or_missing_clipboard_leaves_the_keyword_alone() {
        assert_eq!(substitute("cb", &Fixed(None)), None);
        assert_eq!(substitute("cb", &Fixed(Some(" \n\t "))), None);
        assert_eq!(substitute("1;2", &NoClipboard), None);
    }

    #[test]
    fn oversized_clipboard_is_cut_down() {
        let long = "x".repeat(Limits::MAX_CHARS + 10);
        let source = Fixed(Some(Box::leak(long.into_boxed_str())));
        let resolved = substitute("cb", &source).expect("resolved");
        assert_eq!(resolved.chars().count(), Limits::MAX_CHARS);

        let many: String = (0..Limits::MAX_ARGS + 50)
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(";");
        let source = Fixed(Some(Box::leak(many.into_boxed_str())));
        let resolved = substitute("cb", &source).expect("resolved");
        assert_eq!(resolved.split(';').count(), Limits::MAX_ARGS);
    }
}
