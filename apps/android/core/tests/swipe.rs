//! 滑行输入：解出的字母像点出来的一样进组句。
use std::path::Path;

use qingjian_android::Session;
use qingjian_core::{Engine, SwipeKey, SwipePoint};
use qingjian_dictionary::Dictionary;

fn sample(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../assets/sample")
        .join(name)
}

fn session() -> Session {
    let engine = Engine::new(Dictionary::from_path(sample("dict.tsv")).unwrap());
    let user_dir = std::env::temp_dir().join(format!("qingjian-swipe-{}", std::process::id()));
    Session::new(engine, sample("wubi86.tsv"), sample(""), user_dir)
}

/// 手机全键盘的三排，键 100 × 140。
fn keys() -> Vec<SwipeKey> {
    let rows = [("qwertyuiop", 0.0), ("asdfghjkl", 0.5), ("zxcvbnm", 1.5)];
    rows.iter()
        .enumerate()
        .flat_map(|(row, (letters, indent))| {
            letters
                .chars()
                .enumerate()
                .map(move |(column, letter)| SwipeKey {
                    letter,
                    x: (indent + column as f32 + 0.5) * 100.0,
                    y: (row as f32 + 0.5) * 140.0,
                    width: 100.0,
                    height: 140.0,
                })
        })
        .collect()
}

/// 依次直线经过这几个键的中心。
fn trace(word: &str) -> Vec<SwipePoint> {
    let keys = keys();
    let centres: Vec<SwipePoint> = word
        .chars()
        .map(|letter| {
            let key = keys.iter().find(|key| key.letter == letter).unwrap();
            SwipePoint { x: key.x, y: key.y }
        })
        .collect();
    let mut points = vec![centres[0]];
    for pair in centres.windows(2) {
        for step in 1..=40 {
            let t = step as f32 / 40.0;
            points.push(SwipePoint {
                x: pair[0].x + (pair[1].x - pair[0].x) * t,
                y: pair[0].y + (pair[1].y - pair[0].y) * t,
            });
        }
    }
    points
}

#[test]
fn swipe_composes_like_tapping() {
    let mut swiped = session();
    let swiped = swiped.swipe(&trace("zhe"), &keys());
    let mut tapping = session();
    let tapped = "zhe"
        .chars()
        .map(|letter| tapping.push(letter))
        .last()
        .unwrap();
    assert_eq!(swiped.preedit, tapped.preedit);
    assert!(!swiped.candidates.is_empty());
    assert_eq!(swiped.candidates.len(), tapped.candidates.len());
    assert_eq!(swiped.commit, "");
}
