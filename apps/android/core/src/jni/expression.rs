//! 算式字符的判断：屏幕键盘上的符号进不进算式缓冲区、光标前的字像不像算式结尾，都按 Core 的规则。

use ::jni::JNIEnv;
use ::jni::objects::JObject;
use ::jni::sys::{JNI_FALSE, JNI_TRUE, jboolean, jchar, jlong};

use super::operations::session;

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

/// 算式里按的 `c` 要不要进缓冲区：问的是 Engine 当前的状态，所以带句柄。
///
/// 与 [`Native_expressionChar`] 的区别正是子模式：`;` `:` 不是算式字符，但 `vstat` 后面那个
/// 分隔符要收（`shortcut::modes::accepts_key`）。壳不认这一条，按 `;` 就会先选首选再把分号
/// 上屏，`vstat` / `vbase` 这类子模式在手机上永远打不出来。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_expressionAccepts<'local>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    c: jchar,
) -> jboolean {
    let Some(c) = char::from_u32(u32::from(c)) else {
        return JNI_FALSE;
    };
    if handle == 0 {
        return JNI_FALSE;
    }
    // SAFETY: 句柄来自 `create`，Kotlin 侧只在主线程用、销毁后置 0。
    let handle = unsafe { &mut *(handle as *mut crate::handle::Handle) };
    let accepts = handle.session.expression_accepts(c);
    if accepts { JNI_TRUE } else { JNI_FALSE }
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
