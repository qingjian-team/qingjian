//! 整句实验用的 P2C 打分器：与产品端 `qingjian_neural::P2cScorer` 同一条件，只是推理出错时直接终止。

use qingjian_core::sentence::SentenceScorer;
use qingjian_neural::CharScorer;

pub struct P2cScorer(pub CharScorer);

impl SentenceScorer for P2cScorer {
    fn score(&self, _context: &str, keys: &str, texts: &[&str]) -> Vec<f64> {
        // 实验不能静默回退到基线：任何推理错误立即终止，让不完整结果无法冒充成功。
        self.0
            .score_p2c(keys, texts)
            .expect("P2C evaluation scoring failed")
    }
}
