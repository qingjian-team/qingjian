//! 数据文件位置：只读数据在 `.app/Contents/Resources/`，用户数据在 `~/Library/Application Support/Qingjian/`，按 [`UserDirs`] 分类。

use std::path::PathBuf;

use objc2_foundation::NSBundle;
use qingjian_platform::UserDirs;

use crate::error::HostError;

/// 主 bundle 的 Resources 目录。
pub fn resources_dir() -> Result<PathBuf, HostError> {
    NSBundle::mainBundle()
        .resourcePath()
        .map(|p| PathBuf::from(p.to_string()))
        .ok_or(HostError::NoResources)
}

/// 某个资源文件的完整路径，不存在时报错而不是等到读取时才炸。
pub fn resource(name: &str) -> Result<PathBuf, HostError> {
    let path = resources_dir()?.join(name);
    if path.is_file() {
        Ok(path)
    } else {
        Err(HostError::MissingResource(path))
    }
}

/// 随包的领域词库目录：`.app/Contents/Resources/dicts/`；包里没有就是 `None`。
pub fn bundled_dicts_dir() -> Option<PathBuf> {
    let dir = resources_dir().ok()?.join("dicts");
    dir.is_dir().then_some(dir)
}

/// 随包的辅码码表目录：`.app/Contents/Resources/codes/`；包里没有就是 `None`。
pub fn bundled_codes_dir() -> Option<PathBuf> {
    let dir = resources_dir().ok()?.join("codes");
    dir.is_dir().then_some(dir)
}

/// 随包的形码方案目录 `Resources/schemes/`；包里没有是 `None`。用户方案由
/// [`qingjian_platform::code_schemes`] 在用户数据目录里找。
pub fn bundled_schemes_dir() -> Option<PathBuf> {
    let dir = resources_dir()
        .ok()?
        .join(qingjian_platform::code_schemes::DIR);
    dir.is_dir().then_some(dir)
}

/// 配置文件：`~/Library/Application Support/Qingjian/config.toml`。
pub fn config_file() -> Option<PathBuf> {
    user_data_dir().map(|dir| dir.join("config.toml"))
}

/// 用户数据目录的根，不存在则创建。
pub fn user_data_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support/Qingjian");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// 用户数据的分类布局（下载与缓存也在同一个根下）。
pub fn user_dirs() -> Option<UserDirs> {
    user_data_dir().map(UserDirs::at)
}

/// 用户自己做的形码方案（`schemes/`）与导入的五笔词库（`code_dicts/`）的上一级 `custom/`。
pub fn custom_dir() -> Option<PathBuf> {
    user_dirs().map(|dirs| dirs.custom())
}

/// 用户主题目录 `custom/themes/`，一个主题一个子目录（`<id>/theme.json`），不存在则创建。
pub fn themes_dir() -> Option<PathBuf> {
    existing(user_dirs()?.themes())
}

/// 导入的附加词库 `custom/dicts/`，不存在则创建。
pub fn dicts_dir() -> Option<PathBuf> {
    existing(user_dirs()?.custom_dicts())
}

/// 设置里下载的附加词库 `downloads/dicts/`，不存在则创建。
pub fn downloaded_dicts_dir() -> Option<PathBuf> {
    existing(user_dirs()?.downloaded_dicts())
}

/// 导入的辅码码表 `custom/codes/`，不存在则创建。
pub fn codes_dir() -> Option<PathBuf> {
    existing(user_dirs()?.code_tables())
}

fn existing(dir: PathBuf) -> Option<PathBuf> {
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// 知意选词模型（试验，safetensors 版的整个目录，设置的「实验功能」里下载）：只认 `downloads/models/zhiyi/`。
pub fn zhiyi_model_dir() -> Option<PathBuf> {
    let dir = user_dirs()?.models().join("zhiyi");
    dir.join("model.safetensors").is_file().then_some(dir)
}

/// 含章·通变（P2C，带噪拼音 → 汉字）：设置的「实验功能」里下载到 `downloads/models/hanzhang-tongbian/`；
/// 安装包不再带它，包里的同名目录只给开发时自己放的那份。
pub fn p2c_model_path() -> Option<PathBuf> {
    if let Some(found) = user_dirs()
        .and_then(|dirs| qingjian_neural::find_model(&dirs.models().join("hanzhang-tongbian")))
    {
        return Some(found);
    }
    qingjian_neural::find_model(&resources_dir().ok()?.join("models/hanzhang-tongbian"))
}
