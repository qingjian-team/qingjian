//! 注音（Bopomofo）解码与键盘布局映射。
pub mod decoded;
pub mod layout;
pub mod syllable;

pub(crate) use decoded::decode_at;
pub use decoded::{Decoded, decode};
pub use layout::{Component, map_key};
pub use syllable::ZhuyinSyllable;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_bubu() {
        let d = decode("1j41j4");
        assert_eq!(d.pinyin(), "bu'bu");
        assert_eq!(d.marked(), "ㄅㄨˋㄅㄨˋ");
        assert_eq!(d.units().len(), 2);
        assert!(d.is_complete());
    }

    #[test]
    fn test_decode_incomplete() {
        let d = decode("su"); // ㄋㄧ
        assert_eq!(d.pinyin(), "ni");
        assert_eq!(d.marked(), "ㄋㄧ");
        assert!(!d.is_complete());
    }

    #[test]
    fn test_decode_with_space_as_tone1() {
        let d = decode("su "); // ㄋㄧ (第一聲)
        assert_eq!(d.pinyin(), "ni");
        assert_eq!(d.marked(), "ㄋㄧ"); // 第一聲不顯示標記
        assert!(d.is_complete());
    }

    #[test]
    fn test_decode_zhi() {
        let d = decode("5 "); // ㄓ (第一聲)
        assert_eq!(d.pinyin(), "zhi");
        assert_eq!(d.marked(), "ㄓ");
        assert!(d.is_complete());

        let d2 = decode("5"); // 只有聲母
        assert_eq!(d2.pinyin(), "zh"); // 未完成時作為前綴
        assert!(!d2.is_complete());
    }
}
