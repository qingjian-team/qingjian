//! 双拼给壳的两样东西：设置里列的方案，与键盘上每个键印的声母 / 韵母。都从 Core 的方案表取，壳不写死键位。

mod choice;
mod keycap;

pub use choice::ShuangpinChoice;
pub use keycap::ShuangpinKeycap;

use std::str::FromStr;

use qingjian_core::ShuangpinScheme;

/// 全部方案，按 Core 的顺序（与 PC 设置程序一致）。
pub fn shuangpin_choices() -> Vec<ShuangpinChoice> {
    ShuangpinScheme::ALL
        .into_iter()
        .map(|scheme| ShuangpinChoice {
            key: scheme.key(),
            label: scheme.label(),
        })
        .collect()
}

/// `scheme` 每个键的键帽；认不出的方案为空。字母键按 a–z，用到 `;` 的方案最后多一个 `;`。
pub fn shuangpin_keycaps(scheme: &str) -> Vec<ShuangpinKeycap> {
    let Ok(scheme) = ShuangpinScheme::from_str(scheme) else {
        return Vec::new();
    };
    let semicolon = scheme.uses_semicolon().then_some(';');
    ('a'..='z')
        .chain(semicolon)
        .map(|key| ShuangpinKeycap::of(scheme, key))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xiaohe_keycaps_follow_core_table() {
        let keycaps = shuangpin_keycaps("xiaohe");
        assert_eq!(keycaps.len(), 26);
        let u = &keycaps[usize::from(b'u' - b'a')];
        assert_eq!(u.initial, "sh");
        assert_eq!(u.finals, ["u"]);
        let s = &keycaps[usize::from(b's' - b'a')];
        assert_eq!(s.initial, "");
        assert_eq!(s.finals, ["iong", "ong"]);
    }

    #[test]
    fn semicolon_schemes_get_an_extra_key() {
        assert_eq!(
            shuangpin_keycaps("microsoft").last().map(|k| k.key),
            Some(';')
        );
        assert!(shuangpin_keycaps("nope").is_empty());
        assert_eq!(shuangpin_choices().len(), ShuangpinScheme::ALL.len());
    }
}
