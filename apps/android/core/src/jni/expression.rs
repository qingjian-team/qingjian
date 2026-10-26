//! 算式字符的判断：屏幕键盘上的符号进不进算式缓冲区、光标前的字像不像算式结尾，都按 Core 的规则，不要句柄。

use ::jni::JNIEnv;
use ::jni::objects::JObject;
use ::jni::sys::{JNI_FALSE, JNI_TRUE, jboolean, jchar};

/// 算式里按了 `c`：回要交给 `push` 的字符（`×` `÷` 换成 `*` `/`），不是算式字符回 0。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_expressionChar<'local>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
    c: jchar,
) -> jchar {
    char::from_u32(u32::from(c))
        .and_then(qingjian_core::shortcut::expression_char)
        .and_then(|c| u16::try_from(u32::from(c)).ok())
        .unwrap_or(0)
}

/// 光标前最后一个字是 `c` 时可能是一道算式的结尾，壳才去问补全。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_mayEndCalculation<'local>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
    c: jchar,
) -> jboolean {
    let ends = char::from_u32(u32::from(c)).is_some_and(qingjian_core::may_end_calculation);
    if ends { JNI_TRUE } else { JNI_FALSE }
}
