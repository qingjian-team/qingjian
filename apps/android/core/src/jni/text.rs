//! 文字边界的出口：不要句柄，[`qingjian_core::text`] 的薄包装。
//!
//! 放在这里而不是 Android 的 Kotlin 里，是为了让「删一个词」的边界规则只有一份 ——
//! 做 iOS、鸿蒙那些手机端时直接调同一个 crate，不必各写一套再对不齐。

use ::jni::JNIEnv;
use ::jni::objects::{JObject, JString};
use ::jni::sys::{jboolean, jchar, jint};

/// 光标前那个「词」有多长（按字符算）；读不出字符串时回 0。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_wordTailLength<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    text: JString<'local>,
) -> jint {
    let Ok(text) = env.get_string(&text).map(String::from) else {
        return 0;
    };
    qingjian_core::text::word_tail_length(&text) as jint
}

/// 这个字符算不算「一个词里的字符」（拉丁字母、数字、下划线、连字符、撇号）。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_isWordChar<'local>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
    c: jchar,
) -> jboolean {
    let c = char::from_u32(u32::from(c)).unwrap_or('?');
    u8::from(qingjian_core::text::is_word_char(c))
}
