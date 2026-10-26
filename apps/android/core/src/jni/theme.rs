//! 外观的出口：主题列表、键盘配色、设置页的主题预览（都不要句柄），以及给候选条画笔换主题与字号档。

use std::panic::{AssertUnwindSafe, catch_unwind};

use ::jni::JNIEnv;
use ::jni::objects::{JObject, JString};
use ::jni::sys::{JNI_TRUE, jboolean, jbyteArray, jfloat, jlong, jstring};

use crate::appearance;
use crate::handle::Handle;

/// [`crate::appearance::ThemeEntry`] 列表的 JSON，按 PC 设置程序的顺序。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_themes<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
) -> jstring {
    let Ok(themes) = catch_unwind(appearance::themes) else {
        tracing::error!("列主题时 panic");
        return std::ptr::null_mut();
    };
    string(env, serde_json::to_string(&themes).ok())
}

/// 主题在 `dark` 外观下的键盘配色（[`crate::appearance::KeyboardPalette`] 的 JSON）；没有这个主题时给缺省主题的。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_keyboardPalette<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    theme: JString<'local>,
    dark: jboolean,
) -> jstring {
    let Ok(theme) = env.get_string(&theme).map(String::from) else {
        return std::ptr::null_mut();
    };
    let Ok(palette) = catch_unwind(|| appearance::palette(&theme, dark == JNI_TRUE)) else {
        tracing::error!("推键盘配色时 panic");
        return std::ptr::null_mut();
    };
    string(env, serde_json::to_string(&palette).ok())
}

/// 主题预览：前 8 个字节是宽、高（大端 u32），后面是预乘 RGBA 像素（`Bitmap.copyPixelsFromBuffer` 直接吃）。画不出返回 null。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_themePreview<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    theme: JString<'local>,
    dark: jboolean,
    scale: jfloat,
    text_scale: jfloat,
) -> jbyteArray {
    let Ok(theme) = env.get_string(&theme).map(String::from) else {
        return std::ptr::null_mut();
    };
    let painted = catch_unwind(|| appearance::preview(&theme, dark == JNI_TRUE, scale, text_scale));
    let Ok(Some(pixmap)) = painted else {
        return std::ptr::null_mut();
    };
    let mut bytes = Vec::with_capacity(8 + pixmap.data().len());
    bytes.extend_from_slice(&pixmap.width().to_be_bytes());
    bytes.extend_from_slice(&pixmap.height().to_be_bytes());
    bytes.extend_from_slice(pixmap.data());
    env.byte_array_from_slice(&bytes)
        .map(|array| array.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

/// 候选条换主题与字号档（`text_scale` 是候选字与译文一起的倍数）；壳在键盘建起和设置改了时交。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_setAppearance<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    theme: JString<'local>,
    text_scale: jfloat,
) {
    if handle == 0 {
        return;
    }
    let Ok(theme) = env.get_string(&theme).map(String::from) else {
        return;
    };
    // SAFETY: 同 `super::frame`。
    let handle = unsafe { &mut *(handle as *mut Handle) };
    if let Some(strip) = &mut handle.strip {
        let _ = catch_unwind(AssertUnwindSafe(|| {
            strip.set_appearance(&theme, text_scale)
        }));
    }
}

fn string(env: JNIEnv<'_>, json: Option<String>) -> jstring {
    json.and_then(|json| env.new_string(json).ok())
        .map(JString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}
