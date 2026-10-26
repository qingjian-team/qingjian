package app.qingjian.inputmethod.service.input

import android.text.InputType
import android.view.inputmethod.EditorInfo

/** 密码、网址、邮箱与非文本输入框不过引擎：字母直接上屏，不组句不学习。 */
fun EditorInfo.wantsDirectInput(): Boolean {
    if (inputType and InputType.TYPE_MASK_CLASS != InputType.TYPE_CLASS_TEXT) return true
    return when (inputType and InputType.TYPE_MASK_VARIATION) {
        InputType.TYPE_TEXT_VARIATION_PASSWORD,
        InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD,
        InputType.TYPE_TEXT_VARIATION_WEB_PASSWORD,
        InputType.TYPE_TEXT_VARIATION_URI,
        InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS -> true
        else -> false
    }
}

/**
 * 应用声明这个输入框不要个性化学习（浏览器的无痕标签页等，`IME_FLAG_NO_PERSONALIZED_LEARNING`）：按私密输入处理，
 * 同 PC 上应用声明的私密输入框。
 */
fun EditorInfo.wantsPrivacy(): Boolean = imeOptions and EditorInfo.IME_FLAG_NO_PERSONALIZED_LEARNING != 0
