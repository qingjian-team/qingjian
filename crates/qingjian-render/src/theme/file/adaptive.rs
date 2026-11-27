//! 随外观变的值：写一个值两种外观共用，或写 `{ "light": …, "dark": … }` 各给一个。

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum Adaptive<T> {
    /// 浅色与深色各一个。
    Pair { light: T, dark: T },

    /// 两种外观共用。
    Same(T),
}

impl<T: Copy> Adaptive<T> {
    pub(crate) fn get(&self, dark: bool) -> T {
        match self {
            Self::Pair { light, dark: value } => {
                if dark {
                    *value
                } else {
                    *light
                }
            }
            Self::Same(value) => *value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Adaptive;

    #[test]
    fn a_pair_picks_by_appearance_and_a_single_value_is_shared() {
        let pair: Adaptive<u8> = serde_json::from_str(r#"{"light": 10, "dark": 20}"#).unwrap();
        assert_eq!((pair.get(false), pair.get(true)), (10, 20));
        let same: Adaptive<u8> = serde_json::from_str("30").unwrap();
        assert_eq!((same.get(false), same.get(true)), (30, 30));
    }

    #[test]
    fn a_pair_needs_both_sides() {
        assert!(serde_json::from_str::<Adaptive<u8>>(r#"{"light": 1}"#).is_err());
        assert!(serde_json::from_str::<Adaptive<u8>>(r#"{"dark": 1}"#).is_err());
        assert!(serde_json::from_str::<Adaptive<u8>>("null").is_err());
        let same_float: Adaptive<f32> = serde_json::from_str("1.5").unwrap();
        assert_eq!((same_float.get(false), same_float.get(true)), (1.5, 1.5));
    }
}
