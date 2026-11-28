//! 一条仅保存在内存中的临时验证码。

package app.qingjian.inputmethod.verification

class Code(
    val source: String,
    val id: String,
    val code: String,
    val masked: Boolean,
    val expiresAt: Long,
    val sequence: Long,
)
