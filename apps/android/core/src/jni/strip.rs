//! 候选条位图的出口：先 `renderStrip` 画最新一帧、拿到尺寸与点击区域，壳备好够大的直接缓冲区再 `copyStrip` 拷像素。

use std::panic::{AssertUnwindSafe, catch_unwind};

use ::jni::JNIEnv;
use ::jni::objects::{JByteBuffer, JObject, JString};
use ::jni::sys::{JNI_FALSE, JNI_TRUE, jboolean, jfloat, jint, jlong, jstring};

use crate::handle::Handle;

/// 画最新一帧，返回 [`crate::StripMeta`] 的 JSON；`highlighted` 为负不高亮，`visible_width` 是候选条视口的像素宽。画不出来返回 null，壳退回空白条。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_renderStrip<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    highlighted: jint,
    scale: jfloat,
    dark: jboolean,
    visible_width: jint,
) -> jstring {
    if handle == 0 {
        return std::ptr::null_mut();
    }
    // SAFETY: 同 `super::frame`。
    let handle = unsafe { &mut *(handle as *mut Handle) };
    let Some(strip) = &mut handle.strip else {
        return std::ptr::null_mut();
    };
    let highlighted = usize::try_from(highlighted).ok();
    let painted = catch_unwind(AssertUnwindSafe(|| {
        strip.paint(highlighted, scale, dark == JNI_TRUE)
    }));
    let meta = match painted {
        Ok(Ok(meta)) => meta,
        Ok(Err(error)) => {
            tracing::error!(%error, "候选条画不出来");
            return std::ptr::null_mut();
        }
        Err(_) => {
            tracing::error!("画候选条时 panic");
            return std::ptr::null_mut();
        }
    };
    // 视口里看得见的候选与译文记成「看到过」（生词本的次数）
    let visible: Vec<_> = meta
        .hits
        .iter()
        .filter(|hit| hit.x + hit.width <= visible_width as f32)
        .collect();
    handle.session.note_displayed(&visible);
    let Ok(json) = serde_json::to_string(&meta) else {
        return std::ptr::null_mut();
    };
    env.new_string(json)
        .map(JString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

/// 把上次 `renderStrip` 画的像素拷进壳的直接缓冲区（预乘 RGBA，`Bitmap.copyPixelsFromBuffer` 直接吃）。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_copyStrip<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    buffer: JByteBuffer<'local>,
) -> jboolean {
    if handle == 0 {
        return JNI_FALSE;
    }
    // SAFETY: 同 `super::frame`。
    let handle = unsafe { &mut *(handle as *mut Handle) };
    let Some(strip) = &handle.strip else {
        return JNI_FALSE;
    };
    let (Ok(address), Ok(capacity)) = (
        env.get_direct_buffer_address(&buffer),
        env.get_direct_buffer_capacity(&buffer),
    ) else {
        return JNI_FALSE;
    };
    // SAFETY: 直接缓冲区在这次调用期间由 Java 侧持有，地址与容量来自 JVM。
    let out = unsafe { std::slice::from_raw_parts_mut(address, capacity) };
    match catch_unwind(AssertUnwindSafe(|| strip.copy_pixels(out))) {
        Ok(true) => JNI_TRUE,
        _ => JNI_FALSE,
    }
}
