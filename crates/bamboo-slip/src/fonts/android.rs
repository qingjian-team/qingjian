//! Android 的字体文件清单：AOSP 在 `/system/fonts` 放 Roboto、Noto CJK 与 Noto 彩色 emoji，厂商机型一般也保留这几个文件。
//! 厂商自带的界面字体不在这里；以后读 `/system/etc/fonts.xml` 再补。

use std::path::PathBuf;

use cosmic_text::fontdb::Database;

const SYSTEM_FONTS: &str = "/system/fonts";

/// Noto CJK 集合里各地区那张面的字族名前缀。
const NOTO_CJK: &str = "Noto Sans CJK ";

fn system(names: &[&str]) -> Vec<PathBuf> {
    names
        .iter()
        .map(|name| PathBuf::from(format!("{SYSTEM_FONTS}/{name}")))
        .collect()
}

/// 界面字体：新版 Roboto 是可变字体，老版本才有单独的 Regular。
pub(super) fn ui_fonts() -> Vec<PathBuf> {
    system(&[
        "Roboto-Regular.ttf",
        "RobotoStatic-Regular.ttf",
        "DroidSans.ttf",
    ])
}

/// 界面字体的其余字重：可变字体不用另加；老版本的 Roboto 才分文件。
pub(super) fn ui_weight_fonts() -> Vec<PathBuf> {
    system(&["Roboto-Bold.ttf", "RobotoStatic-Bold.ttf"])
}

/// 汉字：Noto Sans CJK 一个集合里含简繁日韩，加载后由 [`keep_locale_cjk`] 只留 locale 那一张。
pub(super) fn script_fonts(_locale: &str) -> Vec<PathBuf> {
    system(&["NotoSansCJK-Regular.ttc", "NotoSansSC-Regular.otf"])
}

/// 只留 locale 那一张 Noto CJK 面。cosmic-text 在 Android 上没有按 locale 的回退表，
/// 汉字会落到集合里排第一的 JP 面，「骨」「直」画成日本字形。
pub(super) fn keep_locale_cjk(db: &mut Database, locale: &str) {
    let region = match locale {
        l if l.starts_with("ja") => "JP",
        l if l.starts_with("ko") => "KR",
        "zh-HK" => "HK",
        "zh-TW" => "TC",
        _ => "SC",
    };
    let others: Vec<_> = db
        .faces()
        .filter(|face| {
            face.families.iter().any(|(name, _)| {
                name.strip_prefix(NOTO_CJK)
                    .is_some_and(|rest| !rest.starts_with(region))
            })
        })
        .map(|face| face.id)
        .collect();
    for id in others {
        db.remove_face(id);
    }
}

/// 彩色 emoji：Android 13 起是 COLRv1，旧机型是 CBDT 位图。
pub(super) fn emoji_fonts() -> Vec<PathBuf> {
    system(&["NotoColorEmoji.ttf", "NotoColorEmojiLegacy.ttf"])
}
