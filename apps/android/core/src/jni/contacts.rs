//! 通讯录的出口：壳读到系统通讯录后整份交进来。

use ::jni::JNIEnv;
use ::jni::objects::{JObject, JString};
use ::jni::sys::{JNI_FALSE, JNI_TRUE, jboolean, jint, jlong};
use qingjian_core::Contact;

use super::with_session;

/// `contacts` 是 [`Contact`] 数组的 JSON（`[{"name":…,"details":[…]}]`），空数组即关掉；
/// `details` 是上屏名字后给不给电话、邮箱。返回收进来的名字数，解不开或句柄无效返回 -1。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_setContacts<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    contacts: JString<'local>,
    details: jboolean,
) -> jint {
    let Ok(json) = env.get_string(&contacts).map(String::from) else {
        return -1;
    };
    let contacts: Vec<Contact> = match serde_json::from_str(&json) {
        Ok(contacts) => contacts,
        Err(error) => {
            tracing::warn!(%error, "contacts JSON is malformed");
            return -1;
        }
    };
    with_session(handle, |session| {
        session.set_contacts(&contacts, details != JNI_FALSE)
    })
    .map_or(-1, |count| jint::try_from(count).unwrap_or(jint::MAX))
}

/// 候选条上放着电话邮箱或「撤销」（[`crate::session::Session::has_offer`]）：壳的删除键、重输键先收起它们。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_hasOffer<'local>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
) -> jboolean {
    if with_session(handle, |session| session.has_offer()).unwrap_or(false) {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}
