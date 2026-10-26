//! 双拼键盘上一个键印的声韵母。

use qingjian_core::ShuangpinScheme;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ShuangpinKeycap {
    pub key: char,

    /// 键上方的翘舌声母（zh / ch / sh）；声母就是字母本身的不印，为空。
    pub initial: String,

    /// 键下方的韵母，可以有几个（`ong iong`）；不当韵母的键为空。
    pub finals: Vec<String>,
}

impl ShuangpinKeycap {
    pub fn of(scheme: ShuangpinScheme, key: char) -> Self {
        let initial = scheme
            .initial(key)
            .filter(|initial| initial.len() > 1)
            .unwrap_or_default()
            .to_owned();
        let finals = scheme
            .finals(key)
            .iter()
            .map(|final_| (*final_).to_owned())
            .collect();
        Self {
            key,
            initial,
            finals,
        }
    }
}
