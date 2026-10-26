use std::collections::HashMap;

/// 神经分缓存：一组条件（前文 + 用户按键）下各整句文本的神经分。
///
/// 一次查询里整句转换会跑好几遍（纠错变体、中英混输比分……），同一段前文下同一条文本只该问模型一次；
/// 异步打分时查询先把「还没分的文本」攒在 `wanted` 里，壳在停顿后一次送去后台，结果回来按文本填进来，
/// 再查一次就都在缓存里了。前文一变整张表作废。
#[derive(Debug, Default)]
pub(crate) struct NeuralCache {
    /// 这些分数对应的前文。
    context: String,

    /// 这些分数对应的用户按键（P2C 的条件；字级模型用不到，但两者一起构成缓存身份）。
    keys: String,

    /// 文本 → 神经分。
    scores: HashMap<String, f64>,

    /// 还没有分、等着送去后台的文本（不重复）。
    wanted: Vec<String>,

    /// 模型不经词图、直接按按键生成的整句，连同生成时用的那段按键。
    ///
    /// 这段按键与打分用的 `keys` 不是一回事：打分的条件是某一批路径覆盖的那段（英文尾巴那条只覆盖头段），
    /// 生成的条件永远是整段作用域。所以自带一份键，不跟着 [`Self::ensure_condition`] 作废。
    /// 前文不参与：P2C 只认按键。
    generated: Option<(String, Vec<String>)>,

    /// 还没生成、等着送去后台的那段按键。
    generation_wanted: Option<String>,
}

/// 缓存最多留多少条文本：前文不变时一次组句里的候选也就几十条，超过说明有别的东西在刷。
const MAX_ENTRIES: usize = 1024;

impl NeuralCache {
    /// 换条件：前文或按键任一变了就整张清掉。
    pub fn ensure_condition(&mut self, context: &str, keys: &str) {
        if self.context != context || self.keys != keys {
            self.context.clear();
            self.context.push_str(context);
            self.keys.clear();
            self.keys.push_str(keys);
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

    pub fn keys(&self) -> &str {
        &self.keys
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

    /// 这段按键上生成好的整句；还没生成过返回 `None`。
    pub fn generated(&self, keys: &str) -> Option<&[String]> {
        self.generated
            .as_ref()
            .filter(|(cached, _)| cached == keys)
            .map(|(_, texts)| texts.as_slice())
    }

    pub fn insert_generated(&mut self, keys: &str, texts: Vec<String>) {
        self.generation_wanted = None;
        self.generated = Some((keys.to_owned(), texts));
    }

    /// 记下要给这段按键生成；已经生成过的不重复记。
    pub fn want_generation(&mut self, keys: &str) {
        if self.generated(keys).is_none() {
            self.generation_wanted = Some(keys.to_owned());
        }
    }

    pub fn wanted_generation(&self) -> Option<&str> {
        self.generation_wanted.as_deref()
    }

    /// 取走等着送去后台生成的按键。
    pub fn take_wanted_generation(&mut self) -> Option<String> {
        self.generation_wanted.take()
    }
}
