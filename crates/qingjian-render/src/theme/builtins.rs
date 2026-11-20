//! 内置主题：随 crate 编进来的 `theme.json` 与图片，第一次用到时解析一次。

use std::sync::OnceLock;

use super::Theme;

/// 内置主题编进程序的图片：主题里写的路径与文件内容。
pub(super) type EmbeddedFiles = &'static [(&'static str, &'static [u8])];

/// 内置主题：id、源文件与图片，按设置界面列出的顺序；第一个是缺省主题。
pub(super) const BUILTINS: [(&str, &str, EmbeddedFiles); 10] = [
    (
        "qingjian",
        include_str!("../../themes/qingjian/theme.json"),
        &[],
    ),
    (
        "system-blue",
        include_str!("../../themes/system-blue/theme.json"),
        &[],
    ),
    (
        "wechat",
        include_str!("../../themes/wechat/theme.json"),
        &[],
    ),
    (
        "sujian",
        include_str!("../../themes/sujian/theme.json"),
        &[],
    ),
    ("glass", include_str!("../../themes/glass/theme.json"), &[]),
    ("slate", include_str!("../../themes/slate/theme.json"), &[]),
    (
        "sakura",
        include_str!("../../themes/sakura/theme.json"),
        &[
            (
                "images/character.png",
                include_bytes!("../../themes/sakura/images/character.png"),
            ),
            (
                "images/charm.png",
                include_bytes!("../../themes/sakura/images/charm.png"),
            ),
            (
                "images/cat.png",
                include_bytes!("../../themes/sakura/images/cat.png"),
            ),
            (
                "images/blossom.png",
                include_bytes!("../../themes/sakura/images/blossom.png"),
            ),
            (
                "images/bow.svg",
                include_bytes!("../../themes/sakura/images/bow.svg"),
            ),
            (
                "images/crown.svg",
                include_bytes!("../../themes/sakura/images/crown.svg"),
            ),
            (
                "images/heart.svg",
                include_bytes!("../../themes/sakura/images/heart.svg"),
            ),
            (
                "images/sparkle.svg",
                include_bytes!("../../themes/sakura/images/sparkle.svg"),
            ),
            (
                "images/petal.svg",
                include_bytes!("../../themes/sakura/images/petal.svg"),
            ),
            (
                "images/moon.svg",
                include_bytes!("../../themes/sakura/images/moon.svg"),
            ),
            (
                "images/chevron-left.svg",
                include_bytes!("../../themes/sakura/images/chevron-left.svg"),
            ),
            (
                "images/chevron-right.svg",
                include_bytes!("../../themes/sakura/images/chevron-right.svg"),
            ),
        ],
    ),
    (
        "dafeiyu",
        include_str!("../../themes/dafeiyu/theme.json"),
        &[
            (
                "images/girl-side.png",
                include_bytes!("../../themes/dafeiyu/images/girl-side.png"),
            ),
            (
                "images/girl-sleep.png",
                include_bytes!("../../themes/dafeiyu/images/girl-sleep.png"),
            ),
            (
                "images/girl-top.png",
                include_bytes!("../../themes/dafeiyu/images/girl-top.png"),
            ),
            (
                "images/bubble.svg",
                include_bytes!("../../themes/dafeiyu/images/bubble.svg"),
            ),
            (
                "images/chevron-down.svg",
                include_bytes!("../../themes/dafeiyu/images/chevron-down.svg"),
            ),
            (
                "images/chevron-left.svg",
                include_bytes!("../../themes/dafeiyu/images/chevron-left.svg"),
            ),
            (
                "images/chevron-right.svg",
                include_bytes!("../../themes/dafeiyu/images/chevron-right.svg"),
            ),
            (
                "images/heart.svg",
                include_bytes!("../../themes/dafeiyu/images/heart.svg"),
            ),
            (
                "images/keyboard.svg",
                include_bytes!("../../themes/dafeiyu/images/keyboard.svg"),
            ),
            (
                "images/moon.svg",
                include_bytes!("../../themes/dafeiyu/images/moon.svg"),
            ),
            (
                "images/settings.svg",
                include_bytes!("../../themes/dafeiyu/images/settings.svg"),
            ),
            (
                "images/speech-bubble.svg",
                include_bytes!("../../themes/dafeiyu/images/speech-bubble.svg"),
            ),
            (
                "images/waves.svg",
                include_bytes!("../../themes/dafeiyu/images/waves.svg"),
            ),
            (
                "images/whale.svg",
                include_bytes!("../../themes/dafeiyu/images/whale.svg"),
            ),
        ],
    ),
    (
        "miku",
        include_str!("../../themes/miku/theme.json"),
        &[
            (
                "images/miku-side.png",
                include_bytes!("../../themes/miku/images/miku-side.png"),
            ),
            (
                "images/miku-sleep.png",
                include_bytes!("../../themes/miku/images/miku-sleep.png"),
            ),
            (
                "images/miku-top.png",
                include_bytes!("../../themes/miku/images/miku-top.png"),
            ),
            (
                "images/chevron-left.svg",
                include_bytes!("../../themes/miku/images/chevron-left.svg"),
            ),
            (
                "images/chevron-right.svg",
                include_bytes!("../../themes/miku/images/chevron-right.svg"),
            ),
            (
                "images/corner-accent.svg",
                include_bytes!("../../themes/miku/images/corner-accent.svg"),
            ),
            (
                "images/equalizer.svg",
                include_bytes!("../../themes/miku/images/equalizer.svg"),
            ),
            (
                "images/keyboard.svg",
                include_bytes!("../../themes/miku/images/keyboard.svg"),
            ),
            (
                "images/moon.svg",
                include_bytes!("../../themes/miku/images/moon.svg"),
            ),
            (
                "images/music-note.svg",
                include_bytes!("../../themes/miku/images/music-note.svg"),
            ),
            (
                "images/panel-horizontal.svg",
                include_bytes!("../../themes/miku/images/panel-horizontal.svg"),
            ),
            (
                "images/panel-status.svg",
                include_bytes!("../../themes/miku/images/panel-status.svg"),
            ),
            (
                "images/panel-vertical.svg",
                include_bytes!("../../themes/miku/images/panel-vertical.svg"),
            ),
            (
                "images/settings.svg",
                include_bytes!("../../themes/miku/images/settings.svg"),
            ),
        ],
    ),
    (
        "bocchi",
        include_str!("../../themes/bocchi/theme.json"),
        &[
            (
                "images/amp.png",
                include_bytes!("../../themes/bocchi/images/amp.png"),
            ),
            (
                "images/girl-box.png",
                include_bytes!("../../themes/bocchi/images/girl-box.png"),
            ),
            (
                "images/girl-side.png",
                include_bytes!("../../themes/bocchi/images/girl-side.png"),
            ),
            (
                "images/girl-top.png",
                include_bytes!("../../themes/bocchi/images/girl-top.png"),
            ),
            (
                "images/audio-cable.svg",
                include_bytes!("../../themes/bocchi/images/audio-cable.svg"),
            ),
            (
                "images/chevron-left.svg",
                include_bytes!("../../themes/bocchi/images/chevron-left.svg"),
            ),
            (
                "images/chevron-right.svg",
                include_bytes!("../../themes/bocchi/images/chevron-right.svg"),
            ),
            (
                "images/keyboard.svg",
                include_bytes!("../../themes/bocchi/images/keyboard.svg"),
            ),
            (
                "images/note-gold.svg",
                include_bytes!("../../themes/bocchi/images/note-gold.svg"),
            ),
            (
                "images/note.svg",
                include_bytes!("../../themes/bocchi/images/note.svg"),
            ),
            (
                "images/pick.svg",
                include_bytes!("../../themes/bocchi/images/pick.svg"),
            ),
            (
                "images/settings.svg",
                include_bytes!("../../themes/bocchi/images/settings.svg"),
            ),
            (
                "images/star-pink.svg",
                include_bytes!("../../themes/bocchi/images/star-pink.svg"),
            ),
            (
                "images/star.svg",
                include_bytes!("../../themes/bocchi/images/star.svg"),
            ),
        ],
    ),
];

/// 最底层的主题：没写 `extends` 的主题都以它为底，它自己不再往下找。
pub(super) const ROOT_THEME: &str = "qingjian";

/// 解析好的内置主题，第一次用到时解析。内置主题是随包的，解析失败是 bug。
pub(super) fn builtins() -> &'static [Theme] {
    static THEMES: OnceLock<Vec<Theme>> = OnceLock::new();
    THEMES.get_or_init(|| {
        BUILTINS
            .iter()
            .map(|(id, source, files)| {
                let mut theme = Theme::from_json(source, false)
                    .unwrap_or_else(|error| panic!("内置主题 {id} 解析失败：{error}"));
                debug_assert_eq!(theme.id(), *id, "内置主题 id 与目录名不一致");
                theme.embedded = files;
                theme
            })
            .collect()
    })
}

/// 内置主题编进程序的图片；不是内置主题为空。
pub(super) fn builtin_files(id: &str) -> EmbeddedFiles {
    BUILTINS
        .iter()
        .find(|(builtin, ..)| *builtin == id)
        .map_or(&[], |(.., files)| *files)
}

/// `extends` 按 id 找内置主题的源文件。
pub(super) fn builtin_source(id: &str) -> Option<&'static str> {
    BUILTINS
        .iter()
        .find(|(builtin, ..)| *builtin == id)
        .map(|(_, source, _)| *source)
}
