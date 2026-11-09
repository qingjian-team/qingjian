//! 面板目录、常用排序与肤色的出口：都不要句柄。目录在壳第一次打开符号或表情面板时取一次，常用每次打开时取，肤色在长按表情时取。

use std::panic::catch_unwind;
use std::path::Path;

use ::jni::JNIEnv;
use ::jni::objects::{JObject, JString};
use ::jni::sys::jstring;

/// [`crate::CatalogSection`] 列表的 JSON；出错返回 null，壳把面板留空。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_panelCatalog<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
) -> jstring {
    let Ok(json) = catch_unwind(crate::store::catalog::catalog_json) else {
        tracing::error!("取面板目录时 panic");
        return std::ptr::null_mut();
    };
    env.new_string(json)
        .map(JString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

/// 一个 emoji 的五档肤色（JSON 数组，浅到深）；不能接肤色的是空数组。壳长按表情时取。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_skinTones<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    text: JString<'local>,
) -> jstring {
    let Ok(text) = env.get_string(&text).map(String::from) else {
        return std::ptr::null_mut();
    };
    let Ok(tones) = catch_unwind(|| qingjian_core::skin_tone_variants(&text)) else {
        return std::ptr::null_mut();
    };
    let json = serde_json::to_string(&tones).expect("字符串列表总能序列化");
    env.new_string(json)
        .map(JString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

/// 「常用」该显示的条目（JSON 数组），用过的在前；`group` 是「符号」或「表情」。壳每次打开面板时取。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_panelCommon<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    user_dir: JString<'local>,
    group: JString<'local>,
) -> jstring {
    let (Ok(user_dir), Ok(group)) = (
        env.get_string(&user_dir).map(String::from),
        env.get_string(&group).map(String::from),
    ) else {
        return std::ptr::null_mut();
    };
    let Ok(items) = catch_unwind(|| crate::store::catalog::common(Path::new(&user_dir), &group))
    else {
        tracing::error!("排常用时 panic");
        return std::ptr::null_mut();
    };
    let json = serde_json::to_string(&items).expect("字符串列表总能序列化");
    env.new_string(json)
        .map(JString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

/// 面板里点了一格，记一次使用。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_recordPanelUse<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    user_dir: JString<'local>,
    item: JString<'local>,
) {
    let (Ok(user_dir), Ok(item)) = (
        env.get_string(&user_dir).map(String::from),
        env.get_string(&item).map(String::from),
    ) else {
        return;
    };
    if catch_unwind(|| crate::store::catalog::record_use(Path::new(&user_dir), &item)).is_err() {
        tracing::error!("记面板使用时 panic");
    }
}
