//! 主题：一份 `theme.json`（图层树 + 组件 + 颜色变量 + 文字样式）加当前外观（浅色 / 深色）。格式见 `docs/design/theme.md`；
//! 文件可以写注释与尾逗号（见 `jsonc.rs`）。
//!
//! 内置主题随 crate 编进来（`themes/<id>/theme.json`），第一次用到时解析一次；用户主题用 [`Theme::from_json`]。
//! 主题可以 `"extends": "<内置主题 id>"`，只写要改的部分（见 `extends.rs`）。
//! 渲染时引用不到的颜色、样式退回缺省值，加载时 [`validate`] 先把这类问题记成警告。

mod assets;
mod error;
mod extends;
pub(crate) mod file;
mod font;
mod jsonc;
mod library;
mod svg_image;
mod text_sizes;
mod validate;

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use tiny_skia::Pixmap;

use crate::color::Color;

pub use error::ThemeError;
pub(crate) use font::{FamilyId, FontFamilies};
pub use font::{FontSpec, FontWeight};
pub use library::ThemeLibrary;
pub use text_sizes::TextSizes;

use assets::Assets;
use file::{ColorRef, FontRef, LockedAppearance, SCHEMA, ThemeFile};
pub(crate) use svg_image::SvgImage;

/// 内置主题编进程序的图片：主题里写的路径与文件内容。
type EmbeddedFiles = &'static [(&'static str, &'static [u8])];

/// 内置主题：id、源文件与图片，按设置界面列出的顺序；第一个是缺省主题。
const BUILTINS: [(&str, &str, EmbeddedFiles); 4] = [
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
];

/// 最底层的主题：没写 `extends` 的主题都以它为底，它自己不再往下找。
const ROOT_THEME: &str = "qingjian";

/// 引用不到的文字样式退回这个（点）。
const FALLBACK_FONT: FontSpec = FontSpec::new(16.0, 19.0);

#[derive(Debug, Clone)]
pub struct Theme {
    /// 解析后的主题文件，同一主题的浅色 / 深色共用。
    file: Arc<ThemeFile>,

    /// 用深色那一套值。
    dark: bool,

    /// 图片与字体素材（主题目录里的），同一主题的浅色 / 深色共用。
    assets: Arc<Assets>,

    /// 字族表与命名样式，同一主题的浅色 / 深色共用。
    families: Arc<FontFamilies>,

    /// 设置里的字号，盖过主题的。
    sizes: TextSizes,

    /// 设置里关了过渡动画时为 `false`：过渡直接跳到终点，循环动画停在第一帧。
    animations: bool,
}

impl Theme {
    /// 缺省内置主题，浅色。
    pub fn light() -> Self {
        Self::default_builtin().clone()
    }

    /// 缺省内置主题，深色。
    pub fn dark() -> Self {
        Self::default_builtin().with_dark(true)
    }

    /// 按 id 找内置主题；没有这个 id 时为 `None`。
    pub fn builtin(id: &str, dark: bool) -> Option<Self> {
        builtins()
            .iter()
            .find(|theme| theme.id() == id)
            .map(|theme| theme.with_dark(dark))
    }

    /// 全部内置主题（浅色），按设置界面列出的顺序。
    pub fn builtins() -> &'static [Theme] {
        builtins()
    }

    /// 从 `theme.json` 的内容读主题。可以 `extends` 内置主题；引用不到的名字只记警告。
    /// 没写 `extends` 时以青简绿为底（青简绿自己除外）。
    pub fn from_json(json: &str, dark: bool) -> Result<Self, ThemeError> {
        let value = extends::resolve(json, &builtin_source, ROOT_THEME)?;
        let file: ThemeFile = serde_json::from_value(value)?;
        if file.schema > SCHEMA {
            tracing::warn!(
                id = file.meta.id,
                schema = file.schema,
                supported = SCHEMA,
                "主题格式比当前版本新，只按认得的部分画"
            );
        }
        for problem in validate::problems(&file) {
            tracing::warn!(id = file.meta.id, "主题引用有误：{problem}");
        }
        let theme = Self {
            families: Arc::new(FontFamilies::new(&file.text)),
            sizes: TextSizes::default(),
            animations: true,
            file: Arc::new(file),
            dark,
            assets: Arc::default(),
        };
        Ok(theme.with_dark(dark))
    }

    /// 读主题目录：`dir/theme.json` 加它用到的图片与字体（路径相对 `dir`）。
    pub fn from_dir(dir: &Path, dark: bool) -> Result<Self, ThemeError> {
        let json = std::fs::read_to_string(dir.join("theme.json"))?;
        let mut theme = Self::from_json(&json, dark)?;
        let inherited = extends::base_id(&json).map_or(&[][..], |id| builtin_files(&id));
        theme.assets = Arc::new(Assets::load(&theme.file, dir, inherited));
        Ok(theme)
    }

    /// 同一主题换外观；主题锁定了外观（`meta.appearance`）时不换。
    pub fn with_dark(&self, dark: bool) -> Self {
        Self {
            file: Arc::clone(&self.file),
            dark: self.locked_dark().unwrap_or(dark),
            assets: Arc::clone(&self.assets),
            families: Arc::clone(&self.families),
            sizes: self.sizes,
            animations: self.animations,
        }
    }

    /// 同一主题换上设置里的字号。
    pub fn with_text_sizes(&self, sizes: TextSizes) -> Self {
        Self {
            sizes,
            ..self.clone()
        }
    }

    /// 同一主题换上设置里的动画开关。
    pub fn with_animations(&self, animations: bool) -> Self {
        Self {
            animations,
            ..self.clone()
        }
    }

    pub(crate) fn animations(&self) -> bool {
        self.animations
    }

    /// 设置里的候选字号相对主题的比例：表格行高与状态条整体跟着它缩放。
    pub(crate) fn candidate_scale(&self) -> f32 {
        self.sizes.ratio("candidate", |name| self.theme_size(name))
    }

    /// 主题自己写的字号（不算设置里的）：设置界面拿它当字号框的初始值。
    pub fn theme_size(&self, name: &str) -> Option<f32> {
        self.families.style(name).map(|spec| spec.size)
    }

    /// 主题 id。
    pub fn id(&self) -> &str {
        &self.file.meta.id
    }

    /// 显示名。
    pub fn name(&self) -> &str {
        &self.file.meta.name
    }

    pub fn author(&self) -> &str {
        &self.file.meta.author
    }

    /// SPDX 许可证标识。
    pub fn license(&self) -> &str {
        &self.file.meta.license
    }

    /// 样式里写到的系统字族名（不含 `system`）。壳按名字查出字体文件，交给 [`crate::Renderer::load_theme_fonts`]。
    pub fn font_families(&self) -> Vec<String> {
        self.families.names()
    }

    /// 随主题带的字体文件（已检查在主题目录里）。
    pub fn font_files(&self) -> &[PathBuf] {
        self.assets.fonts()
    }

    fn default_builtin() -> &'static Theme {
        &builtins()[0]
    }

    /// 锁定的外观：`Some(true)` 只有深色，`Some(false)` 只有浅色。
    pub(crate) fn locked_dark(&self) -> Option<bool> {
        self.file
            .meta
            .appearance
            .map(|appearance| appearance == LockedAppearance::Dark)
    }

    pub(crate) fn file(&self) -> &ThemeFile {
        &self.file
    }

    /// 颜色引用按当前外观取值；变量不存在时透明。
    pub(crate) fn color(&self, color: &ColorRef) -> Color {
        match color {
            ColorRef::Literal(color) => *color,
            ColorRef::Variable(name) => self
                .file
                .variables
                .get(name)
                .map_or(Color::rgba(0, 0, 0, 0), |value| value.get(self.dark).0),
        }
    }

    /// 主题里写的图片路径对应的位图；没有、没读进来或是 SVG 时为 `None`。
    pub(crate) fn image(&self, path: &str) -> Option<Arc<Pixmap>> {
        self.assets.image(path)
    }

    /// 主题里写的 SVG 图片；没有、没读进来或不是 SVG 时为 `None`。
    pub(crate) fn svg(&self, path: &str) -> Option<Arc<SvgImage>> {
        self.assets.svg(path)
    }

    /// 命名文字样式（点）。
    pub(crate) fn font(&self, name: &str) -> FontSpec {
        let spec = self.families.style(name).unwrap_or(FALLBACK_FONT);
        text_sizes::scaled(spec, self.sizes.ratio(name, |base| self.theme_size(base)))
    }

    /// 字族表，渲染器按 [`FamilyId`] 查回退链；两份 `Theme` 共用同一张表时指针相同。
    pub(crate) fn families(&self) -> &Arc<FontFamilies> {
        &self.families
    }

    /// 节点里的 `font`：样式名，或在样式上改字号 / 字重。
    pub(crate) fn font_ref(&self, font: &FontRef) -> FontSpec {
        font.apply(font.base().map_or(FALLBACK_FONT, |name| self.font(name)))
    }

    /// 当前外观下的文字覆盖率 gamma。
    pub(crate) fn text_gamma(&self) -> f32 {
        self.file.text.gamma.get(self.dark)
    }
}

/// 解析好的内置主题，第一次用到时解析。内置主题是随包的，解析失败是 bug。
fn builtins() -> &'static [Theme] {
    static THEMES: OnceLock<Vec<Theme>> = OnceLock::new();
    THEMES.get_or_init(|| {
        BUILTINS
            .iter()
            .map(|(id, source, files)| {
                let mut theme = Theme::from_json(source, false)
                    .unwrap_or_else(|error| panic!("内置主题 {id} 解析失败：{error}"));
                debug_assert_eq!(theme.id(), *id, "内置主题 id 与目录名不一致");
                theme.assets = Arc::new(Assets::embedded(&theme.file, files));
                theme
            })
            .collect()
    })
}

/// 内置主题编进程序的图片；不是内置主题为空。
fn builtin_files(id: &str) -> EmbeddedFiles {
    BUILTINS
        .iter()
        .find(|(builtin, ..)| *builtin == id)
        .map_or(&[], |(.., files)| *files)
}

/// `extends` 按 id 找内置主题的源文件。
fn builtin_source(id: &str) -> Option<&'static str> {
    BUILTINS
        .iter()
        .find(|(builtin, ..)| *builtin == id)
        .map(|(_, source, _)| *source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_themes_parse_without_problems() {
        for theme in Theme::builtins() {
            assert!(
                validate::problems(theme.file()).is_empty(),
                "{}: {:?}",
                theme.id(),
                validate::problems(theme.file())
            );
        }
        assert_eq!(Theme::builtins().len(), BUILTINS.len());
        let theme = Theme::light();
        assert_eq!(theme.id(), "qingjian");
        assert!(Theme::builtin("wechat", true).is_some());
        assert!(Theme::builtin("nope", false).is_none());
        assert_eq!(
            theme.color(&ColorRef::Variable("accent".to_owned())),
            Color::rgba(176, 206, 125, 127)
        );
        assert_eq!(
            Theme::dark().color(&ColorRef::Variable("accent".to_owned())),
            Color::rgb(36, 76, 36)
        );
    }

    #[test]
    fn theme_file_accepts_comments_trailing_commas_and_schema_key() {
        let theme = Theme::from_json(
            r##"{
                "$schema": "https://qingjian.app/schema/theme-1.json",
                "extends": "qingjian", // 以青简绿为底
                "schema": 1,
                "meta": { "id": "jsonc", "name": "注释", },
                /* 只改高亮色 */
                "variables": { "accent": "#ffb7d5", },
            }"##,
            false,
        )
        .unwrap();
        assert_eq!(theme.id(), "jsonc");
        assert_eq!(
            theme.color(&ColorRef::Variable("accent".to_owned())),
            Color::rgb(0xff, 0xb7, 0xd5)
        );
    }

    #[test]
    fn builtin_images_are_all_embedded() {
        for theme in Theme::builtins() {
            for path in assets::image_paths(theme.file()) {
                assert!(
                    theme.image(&path).is_some() || theme.svg(&path).is_some(),
                    "内置主题 {} 用到的 {path} 没有编进程序",
                    theme.id()
                );
            }
        }
    }

    #[test]
    fn user_theme_inherits_builtin_images() {
        let dir = std::env::temp_dir().join("qingjian-theme-inherits-sakura");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("theme.json"),
            r##"{ "extends": "sakura", "schema": 1, "meta": { "id": "my-sakura", "name": "我的樱花" },
                 "variables": { "text": "#000000" } }"##,
        )
        .unwrap();
        let theme = Theme::from_dir(&dir, false).unwrap();
        assert!(theme.image("images/character.png").is_some());
        assert!(theme.svg("images/heart.svg").is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn user_theme_without_extends_builds_on_qingjian() {
        let theme = Theme::from_json(
            r##"{ "schema": 1, "meta": { "id": "red", "name": "红" }, "variables": { "accent": "#ff0000" } }"##,
            false,
        )
        .unwrap();
        assert_eq!(
            theme.color(&ColorRef::Variable("accent".to_owned())),
            Color::rgb(255, 0, 0)
        );
        assert_eq!(theme.font("candidate"), Theme::light().font("candidate"));
    }

    #[test]
    fn text_sizes_scale_styles_and_rows() {
        let theme = Theme::light();
        let (candidate, index, annotation) = (
            theme.font("candidate"),
            theme.font("index"),
            theme.font("annotation"),
        );
        let ratio = 26.0 / candidate.size;
        let sized = theme.with_text_sizes(TextSizes::new(26.0, 0.0));
        assert_eq!(sized.font("candidate").size, 26.0);
        assert_eq!(
            sized.font("candidate").line_height,
            candidate.line_height * ratio
        );
        assert_eq!(sized.font("index").size, index.size * ratio);
        assert_eq!(sized.font("annotation"), annotation);
        assert_eq!(sized.candidate_scale(), ratio);
        // 换外观不丢设置里的字号
        assert_eq!(sized.with_dark(true).font("candidate").size, 26.0);
        // 不是正数当没填
        assert_eq!(TextSizes::new(-1.0, f32::NAN), TextSizes::default());
    }
}
