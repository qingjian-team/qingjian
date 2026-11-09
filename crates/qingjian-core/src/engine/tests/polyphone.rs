//! 多音字的少见读音按读音占比扣分，结构上打平的切分按整句得分挑。

use super::*;

/// 按字形给分的假模型：区 比 偶 常见得多（它的 qu 读音撑起来的），上 偶 连着读不如整词 山沟。
struct GlyphModel;

impl LanguageModel for GlyphModel {
    fn log_prob(&self, previous: Option<&str>, word: &str) -> Option<f64> {
        match (previous, word) {
            (_, "区") => Some(-3.0),
            (Some("上"), "偶") => Some(-4.0),
            (_, "偶") => Some(-5.0),
            (None, "上") => Some(-2.0),
            (None, "山沟") => Some(-5.0),
            _ => None,
        }
    }
}

const POLYPHONES: &str = "区\tqu\t170000\n区\tou\t8500\n偶\tou\t15000\n上\tshang\t50000\n山\tshan\t9000\n沟\tgou\t5000\n山沟\tshan gou\t400\n";

fn engine() -> Engine {
    Engine::new(Dictionary::parse(POLYPHONES).unwrap()).with_language_model(Box::new(GlyphModel))
}

#[test]
fn a_rare_reading_does_not_take_the_glyph_score() {
    let mut engine = engine();
    engine.set_input("ou");
    assert_eq!(texts_of(&engine)[..2], ["偶", "区"]);
}

#[test]
fn tied_segmentations_are_settled_by_the_sentence_score() {
    // shang ou 与 shan gou 都是两个完整音节，切分器按贪心把 shang ou 排前面
    let mut engine = engine();
    engine.set_input("shangou");
    let items = engine.query().unwrap().candidates.items;
    assert_eq!(items[0].text, "山沟");
    assert_eq!(items[0].kind, CandidateKind::Chinese);
}
