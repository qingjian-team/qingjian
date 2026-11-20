//! 主题：一份 `theme.json`（图层树 + 组件 + 颜色变量 + 文字样式）加当前外观（浅色 / 深色）。格式见 `docs/design/theme.md`；
//! 文件可以写注释与尾逗号（见 `jsonc.rs`）。
//!
//! 内置主题随 crate 编进来（`themes/<id>/theme.json`），第一次用到时解析一次；用户主题用 [`Theme::from_json`]。
//! 主题可以 `"extends": "<内置主题 id>"`，只写要改的部分（见 `extends.rs`）。
//! 渲染时引用不到的颜色、样式退回缺省值，加载时 [`validate`] 先把这类问题记成警告。

mod assets;
mod builtins;
mod colors;
mod error;
mod extends;
pub(crate) mod file;
mod font;
mod fonts;
mod jsonc;
mod keyboard;
mod library;
mod meta;
mod text_sizes;
mod validate;

#[cfg(test)]
mod characterize;
#[cfg(test)]
mod tests;

use std::path::Path;
use std::sync::{Arc, OnceLock};

use bamboo_slip::FontWeight;
use font::FontFamilies;

pub use error::ThemeError;
pub use keyboard::KeyboardColors;
pub use library::ThemeLibrary;
pub use text_sizes::TextSizes;

use assets::Assets;
use builtins::{ROOT_THEME, builtin_files, builtin_source, builtins};
use file::{SCHEMA, ThemeFile};

/// 解析后的主题文件，同一主题的浅色 / 深色共用。
#[derive(Debug, Clone)]
pub struct Theme {
    file: Arc<ThemeFile>,

    /// 用深色那一套值。
    dark: bool,

    /// 图片与字体素材，同一主题的浅色 / 深色共用。用户主题读目录时就填好；
    /// 内置主题第一次画到才从 `embedded` 解码：内置主题有好几个带图的，而一次只画一个。
    assets: Arc<OnceLock<Assets>>,

    /// 内置主题编进程序的图片；别的主题为空。
    embedded: builtins::EmbeddedFiles,

    /// 字族表与命名样式，同一主题的浅色 / 深色共用。
    families: Arc<FontFamilies>,

    /// 设置里的字号，盖过主题的。
    sizes: TextSizes,

    /// 设置里的候选字字重，盖过主题 `candidate` 样式的；节点上自己写了字重的（键位提示的键帽字）不动。
    candidate_weight: Option<FontWeight>,

    /// 设置里选了字体：主题写死的正文字族让给它（见 [`Theme::with_user_font`]）。
    user_font: bool,

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
            families: Arc::new(FontFamilies::new(&file.text, false)),
            sizes: TextSizes::default(),
            candidate_weight: None,
            user_font: false,
            animations: true,
            file: Arc::new(file),
            dark,
            assets: Arc::default(),
            embedded: &[],
        };
        Ok(theme.with_dark(dark))
    }

    /// 读主题目录：`dir/theme.json` 加它用到的图片与字体（路径相对 `dir`）。
    pub fn from_dir(dir: &Path, dark: bool) -> Result<Self, ThemeError> {
        let json = std::fs::read_to_string(dir.join("theme.json"))?;
        let mut theme = Self::from_json(&json, dark)?;
        let inherited = extends::base_id(&json).map_or(&[][..], |id| builtin_files(&id));
        theme.assets = Arc::new(OnceLock::from(Assets::load(&theme.file, dir, inherited)));
        Ok(theme)
    }

    /// 同一主题换外观；主题锁定了外观（`meta.appearance`）时不换。
    pub fn with_dark(&self, dark: bool) -> Self {
        Self {
            file: Arc::clone(&self.file),
            dark: self.locked_dark().unwrap_or(dark),
            assets: Arc::clone(&self.assets),
            embedded: self.embedded,
            families: Arc::clone(&self.families),
            sizes: self.sizes,
            candidate_weight: self.candidate_weight,
            user_font: self.user_font,
            animations: self.animations,
        }
    }

    /// 同一主题换上设置里的字号。
    pub fn with_text_sizes(&self, sizes: TextSizes) -> Self {
        Self { sizes, ..self.clone() }
    }

    /// 同一主题换上设置里的候选字字重（100–900）；`None` 用主题的。
    pub fn with_candidate_weight(&self, weight: Option<u16>) -> Self {
        Self {
            candidate_weight: weight.map(FontWeight),
            ..self.clone()
        }
    }

    /// 设置里选了字体时传 `true`（壳已把那个字体登记成 `system`）：主题的缺省字族与候选、注释、序号、拼音的字族都换成它，
    /// 否则写死了字族的主题（樱花的宋体、大肥鱼的苹方 / 微软雅黑）里选字体没有效果（#957）。
    pub fn with_user_font(&self, user_font: bool) -> Self {
        if user_font == self.user_font {
            return self.clone();
        }
        Self {
            families: Arc::new(FontFamilies::new(&self.file.text, user_font)),
            user_font,
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

    fn default_builtin() -> &'static Theme {
        &builtins()[0]
    }
}
