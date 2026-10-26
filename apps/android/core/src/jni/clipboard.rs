//! 剪贴板与常用语面板的出口：请求是 JSON（`op` 区分），回的是操作之后的整份列表 JSON；出错返回 null，壳当作没变。
//! 拆词与重挂常用语要用引擎，带句柄。

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use ::jni::JNIEnv;
use ::jni::objects::{JByteArray, JObject, JString};
use ::jni::sys::{jint, jlong, jstring};
use serde::Serialize;

use super::with_session;
use crate::{ClipboardRequest, PhraseRequest};

fn to_java<'local>(env: &JNIEnv<'local>, value: &impl Serialize) -> jstring {
    let json = serde_json::to_string(value).expect("面板数据总能序列化");
    env.new_string(json)
        .map(JString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

fn limit(value: jint) -> usize {
    usize::try_from(value).unwrap_or(qingjian_learning::DEFAULT_CLIPBOARD_LIMIT)
}

/// [`crate::clipboard`]：`request` 是 [`ClipboardRequest`] 的 JSON，`limit` 是设置里的最大条数。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_clipboard<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    user_dir: JString<'local>,
    request: JString<'local>,
    max: jint,
) -> jstring {
    let (Ok(user_dir), Ok(request)) = (
        env.get_string(&user_dir).map(String::from),
        env.get_string(&request).map(String::from),
    ) else {
        return std::ptr::null_mut();
    };
    let Ok(request) = serde_json::from_str::<ClipboardRequest>(&request) else {
        tracing::warn!(request, "看不懂的剪贴板请求");
        return std::ptr::null_mut();
    };
    match catch_unwind(|| crate::clipboard(Path::new(&user_dir), request, limit(max))) {
        Ok(items) => to_java(&env, &items),
        Err(_) => std::ptr::null_mut(),
    }
}

/// 记一张复制的图片：`bytes` 是图片文件内容，`extension` 如 `png`。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_clipboardAddImage<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    user_dir: JString<'local>,
    bytes: JByteArray<'local>,
    extension: JString<'local>,
    max: jint,
) -> jstring {
    let (Ok(user_dir), Ok(extension), Ok(bytes)) = (
        env.get_string(&user_dir).map(String::from),
        env.get_string(&extension).map(String::from),
        env.convert_byte_array(&bytes),
    ) else {
        return std::ptr::null_mut();
    };
    match catch_unwind(|| {
        crate::clipboard_add_image(Path::new(&user_dir), &bytes, &extension, limit(max))
    }) {
        Ok(items) => to_java(&env, &items),
        Err(_) => std::ptr::null_mut(),
    }
}

/// [`crate::phrases`]：`request` 是 [`PhraseRequest`] 的 JSON。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_phrases<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    user_dir: JString<'local>,
    request: JString<'local>,
) -> jstring {
    let (Ok(user_dir), Ok(request)) = (
        env.get_string(&user_dir).map(String::from),
        env.get_string(&request).map(String::from),
    ) else {
        return std::ptr::null_mut();
    };
    let Ok(request) = serde_json::from_str::<PhraseRequest>(&request) else {
        tracing::warn!(request, "看不懂的常用语请求");
        return std::ptr::null_mut();
    };
    match catch_unwind(|| crate::phrases(Path::new(&user_dir), request)) {
        Ok(page) => to_java(&env, &page),
        Err(_) => std::ptr::null_mut(),
    }
}

/// [`crate::PhraseRules`] 的 JSON：常用语编辑框的缺省分组与输入码长度。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_phraseRules<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
) -> jstring {
    match catch_unwind(crate::phrase_rules) {
        Ok(rules) => to_java(&env, &rules),
        Err(_) => std::ptr::null_mut(),
    }
}

/// 拆词：一段文字切成可点选的小块（JSON 字符串数组）。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_textPieces<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    text: JString<'local>,
) -> jstring {
    let Ok(text) = env.get_string(&text).map(String::from) else {
        return std::ptr::null_mut();
    };
    match with_session(handle, |session| session.pieces(&text)) {
        Some(pieces) => to_java(&env, &pieces),
        None => std::ptr::null_mut(),
    }
}

/// 常用语改过之后重挂带输入码的那些。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_reloadPhrases<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    user_dir: JString<'local>,
) {
    let Ok(user_dir) = env.get_string(&user_dir).map(String::from) else {
        return;
    };
    let _ = catch_unwind(AssertUnwindSafe(|| {
        with_session(handle, |session| {
            session.reload_phrases(Path::new(&user_dir))
        })
    }));
}

/// 剪贴板卡片的代码显示（[`crate::CodeView`] 的 JSON）：像不像代码、高亮片段（UTF-16 偏移）。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_codeView<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    text: JString<'local>,
) -> jstring {
    let Ok(text) = env.get_string(&text).map(String::from) else {
        return std::ptr::null_mut();
    };
    match catch_unwind(|| crate::code_view(&text)) {
        Ok(view) => to_java(&env, &view),
        Err(_) => std::ptr::null_mut(),
    }
}
