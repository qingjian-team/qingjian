use objc2_foundation::{NSBundle, NSString};

/// 从主 bundle 的 Info.plist 读出的输入法身份信息。
#[derive(Debug, Clone)]
pub struct BundleInfo {
    /// `InputMethodConnectionName`，IMK 用它注册 NSConnection。
    pub connection_name: String,

    /// `CFBundleIdentifier`。
    pub identifier: String,

    /// `CFBundleShortVersionString`，菜单里显示。
    pub version: String,

    /// 构建标识：打包脚本通过环境变量 `QINGJIAN_BUILD` 塞进来的 git 短哈希与日期；直接 `cargo build` 的是「本地构建」。
    pub build: String,
}

impl BundleInfo {
    /// 读不到时回退到编译期常量，方便在 `.app` 之外直接跑二进制看日志。
    pub fn from_main_bundle() -> Self {
        let bundle = NSBundle::mainBundle();
        let identifier = bundle
            .bundleIdentifier()
            .map(|s| s.to_string())
            .unwrap_or_else(|| DEFAULT_IDENTIFIER.to_owned());
        let connection_name = bundle
            .objectForInfoDictionaryKey(&NSString::from_str("InputMethodConnectionName"))
            .and_then(|value| value.downcast::<NSString>().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{identifier}_Connection"));
        let version = bundle
            .objectForInfoDictionaryKey(&NSString::from_str("CFBundleShortVersionString"))
            .and_then(|value| value.downcast::<NSString>().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_owned());
        Self {
            connection_name,
            identifier,
            version,
            build: option_env!("QINGJIAN_BUILD")
                .unwrap_or("本地构建")
                .to_owned(),
        }
    }
}

/// 与 `Info.plist` 里的 `CFBundleIdentifier` 保持一致。
pub(crate) const DEFAULT_IDENTIFIER: &str = "app.qingjian.inputmethod";
