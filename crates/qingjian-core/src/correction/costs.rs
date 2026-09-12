use super::TypoKind;

/// 敲错纠正的代价（log 概率的扣分）。缺省值是现在用的那组，回放调参（`qingjian-cli --tune`）时可以整组换掉，
/// 引擎与壳只用缺省值。
///
/// 音节级的四类代价叠在词的得分上：候选拼音与敲的不同时，要比原样的解释好这么多才排得过。
/// 换位与相邻键最常见、代价最低；多敲少敲稍贵。数值按「一个音节敲错的先验约 1%，再分摊到它的十几个变体上」定，
/// 与整段一处编辑的代价同一量级；定太低时常用词会借着个人词频从敲错边挤掉用户真要的生僻词。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypoCosts {
    /// 相邻两键敲反了（`shou` → `shuo`）。
    pub transpose: f64,

    /// 敲到了旁边的键（`ni` → `mi`）。
    pub substitute: f64,

    /// 多敲了一个键（`gang` → `gan`）。
    pub extra: f64,

    /// 少敲了一个键（`gan` → `guan`）。
    pub missing: f64,

    /// 个人敲错表最多给一条边减多少代价：减到只剩 1.0 左右，敲错的解释仍要比原样好一点才排上来。
    pub discount_cap: f64,

    /// 整段一处编辑的纠错代价：纠正后的整句得分要比原样转出的高出这么多才纠。
    /// 相当于「敲错一个键」的先验约 1/150；原样是合法简拼（`nhao` → 你好）时两边路径一样，纠正不会赢。
    pub correction_penalty: f64,
}

impl TypoCosts {
    /// 现在用的那组。
    pub const DEFAULT: Self = Self {
        transpose: 5.0,
        substitute: 5.0,
        extra: 5.5,
        missing: 5.5,
        discount_cap: 3.0,
        correction_penalty: 5.0,
    };

    /// 这类敲错的基础代价。
    pub fn cost(&self, kind: TypoKind) -> f64 {
        match kind {
            TypoKind::Transpose => self.transpose,
            TypoKind::Substitute => self.substitute,
            TypoKind::Extra => self.extra,
            TypoKind::Missing => self.missing,
        }
    }

    /// 词图里一条敲错边的代价：类别的基础代价按个人敲错表打折（见 [`Self::discounted`]）。
    pub fn typo_cost(&self, kind: TypoKind, accepted: u32) -> f64 {
        self.discounted(self.cost(kind), accepted)
    }

    /// 整段一处编辑的纠错代价，按个人敲错表打折。
    pub fn correction_cost(&self, accepted: u32) -> f64 {
        self.discounted(self.correction_penalty, accepted)
    }

    /// `base` 代价减去个人折扣：折扣 = min(ln(1 + 接受过的次数), [`Self::discount_cap`])。
    pub fn discounted(&self, base: f64, accepted: u32) -> f64 {
        base - (1.0 + f64::from(accepted)).ln().min(self.discount_cap)
    }
}

impl Default for TypoCosts {
    fn default() -> Self {
        Self::DEFAULT
    }
}
