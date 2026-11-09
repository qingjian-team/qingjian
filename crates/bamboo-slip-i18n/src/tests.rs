use crate::locale::normalize;
use crate::{Catalog, set_locale};

static CATALOG: Catalog = Catalog::new(&[
    (
        "zh-Hans",
        "[meta]\nname = \"简体中文\"\n[page]\ntitle = \"设置\"\ncount = \"共 {n} 个\"\nnew = \"新功能\"\n",
    ),
    (
        "en",
        "[meta]\nname = \"English\"\n[page]\ntitle = \"Settings\"\ncount = \"{n} in total\"\nnew = \"New\"\n",
    ),
    (
        "vi",
        "[meta]\nname = \"Tiếng Việt\"\n[page]\ntitle = \"Cài đặt\"\n",
    ),
]);

// 界面语言是进程级的，几种情形放在一个测试里按顺序走，免得并行的测试互相改掉
#[test]
fn lookup_follows_the_locale_and_falls_back() {
    set_locale("vi-VN");
    assert_eq!(CATALOG.get("page.title"), "Cài đặt");
    assert_eq!(CATALOG.get("page.new"), "New");
    assert_eq!(CATALOG.get("page.missing"), "page.missing");

    set_locale("zh-TW");
    assert_eq!(CATALOG.get("page.title"), "Settings");
    assert_eq!(CATALOG.format("page.count", &[("n", &3)]), "3 in total");

    set_locale("zh-CN");
    assert_eq!(CATALOG.get("page.title"), "设置");
    assert_eq!(CATALOG.format("page.count", &[("n", &3)]), "共 3 个");
}

#[test]
fn languages_are_named_in_their_own_language() {
    assert_eq!(
        CATALOG.languages(),
        vec![
            ("zh-Hans", "简体中文"),
            ("en", "English"),
            ("vi", "Tiếng Việt")
        ]
    );
}

#[test]
fn chinese_tags_collapse_to_script() {
    assert_eq!(normalize("zh-Hant-TW"), "zh-Hant");
    assert_eq!(normalize("zh_HK.UTF-8"), "zh-Hant");
    assert_eq!(normalize("zh-CN"), "zh-Hans");
    assert_eq!(normalize("zh"), "zh-Hans");
    assert_eq!(normalize("vi_VN.UTF-8"), "vi-VN");
    assert_eq!(normalize("en-US"), "en-US");
}

#[test]
fn problems_report_stray_keys_placeholders_and_gaps_in_english() {
    static BROKEN: Catalog = Catalog::new(&[
        ("zh-Hans", "a = \"甲 {n}\"\nb = \"乙\"\n"),
        ("en", "a = \"A {count}\"\n"),
        ("de", "a = \"A {n}\"\nz = \"Z\"\n"),
        ("ja", "a = "),
    ]);
    let problems = BROKEN.problems();
    assert_eq!(problems.len(), 4, "{problems:?}");
    assert!(problems.contains(&"de: unknown key z".to_owned()));
    assert!(problems.contains(&"en: missing key b".to_owned()));
    assert!(problems.contains(&"en: unknown placeholder in a".to_owned()));
    assert!(problems.iter().any(|problem| problem.starts_with("ja: ")));
    assert!(CATALOG.problems().is_empty());
}
