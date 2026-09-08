/// 评测集里的一条：一句汉字、它的全拼、句子前面的真实上文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pair {
    /// 要还原的句子（纯汉字）。
    pub text: String,

    /// 无分隔的全拼，和用户真敲的形态一致。
    pub pinyin: String,

    /// 同一段落里这句之前的真实文本（含标点），最多 [`MAX_CONTEXT_CHARS`] 个字符；给整句转换当上文。
    pub context: String,
}

/// 上文最多留几个字符。
pub const MAX_CONTEXT_CHARS: usize = 64;

impl Pair {
    /// 冻结文件里的一行：`句子\t拼音\t上文`（上文可空）。
    pub fn parse(line: &str) -> Option<Self> {
        let mut fields = line.split('\t');
        let text = fields.next()?.trim();
        let pinyin = fields.next()?.trim();
        if text.is_empty() || pinyin.is_empty() {
            return None;
        }
        let context = fields.next().unwrap_or("").trim().to_owned();
        Some(Self {
            text: text.to_owned(),
            pinyin: pinyin.to_owned(),
            context,
        })
    }

    pub fn to_line(&self) -> String {
        format!("{}\t{}\t{}", self.text, self.pinyin, self.context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_writes_the_same_line() {
        let pair = Pair::parse("我想学好\twoxiangxuehao\t今天，").unwrap();
        assert_eq!(pair.text, "我想学好");
        assert_eq!(pair.pinyin, "woxiangxuehao");
        assert_eq!(pair.context, "今天，");
        assert_eq!(pair.to_line(), "我想学好\twoxiangxuehao\t今天，");
        assert!(Pair::parse("只有一列").is_none());
    }
}
