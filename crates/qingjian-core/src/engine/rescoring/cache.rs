use std::collections::HashMap;

/// 神经分缓存：一段前文下各整句文本的 `log P(文本 | 前文)`。
///
/// 一次查询里整句转换会跑好几遍（纠错变体、中英混输比分……），同一段前文下同一条文本只该问模型一次；
/// 异步打分时查询先把「还没分的文本」攒在 `wanted` 里，壳在停顿后一次送去后台，结果回来按文本填进来，
/// 再查一次就都在缓存里了。前文一变整张表作废。
#[derive(Debug, Default)]
pub(crate) struct NeuralCache {
    /// 这些分数对应的前文。
    context: String,

    /// 文本 → 神经分。
    scores: HashMap<String, f64>,

    /// 还没有分、等着送去后台的文本（不重复）。
    wanted: Vec<String>,
}

/// 缓存最多留多少条文本：前文不变时一次组句里的候选也就几十条，超过说明有别的东西在刷。
const MAX_ENTRIES: usize = 1024;

impl NeuralCache {
    /// 换前文：不一样就整张清掉。
    pub fn ensure_context(&mut self, context: &str) {
        if self.context != context {
            self.context.clear();
            self.context.push_str(context);
            self.scores.clear();
            self.wanted.clear();
        }
        if self.scores.len() > MAX_ENTRIES {
            self.scores.clear();
        }
    }

    pub fn context(&self) -> &str {
        &self.context
    }

    pub fn get(&self, text: &str) -> Option<f64> {
        self.scores.get(text).copied()
    }

    pub fn insert(&mut self, text: &str, score: f64) {
        self.scores.insert(text.to_owned(), score);
    }

    /// 记下一条要分的文本；已经有分或已经在等的不重复记。
    pub fn want(&mut self, text: &str) {
        if !self.scores.contains_key(text) && !self.wanted.iter().any(|w| w == text) {
            self.wanted.push(text.to_owned());
        }
    }

    pub fn has_wanted(&self) -> bool {
        !self.wanted.is_empty()
    }

    /// 取走等着送去后台的文本。
    pub fn take_wanted(&mut self) -> Vec<String> {
        std::mem::take(&mut self.wanted)
    }
}
