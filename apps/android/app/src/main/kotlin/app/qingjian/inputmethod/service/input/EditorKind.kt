package app.qingjian.inputmethod.service.input

import android.text.InputType
import android.view.inputmethod.EditorInfo

/**
 * 密码、邮箱与非文本输入框不过引擎：字母直接上屏，不组句不学习。
 * 网址框照常组句：浏览器地址栏也是搜索框，要能打中文，见 [wantsPlainPunctuation]。
 *
 * 整个 `inputType` 是 0（终端一类要原始按键的）才算没有文本；类别是 0 却带着文本标志的照常组句：
 * 抖音直播间的输入框报 `0x20000`（只有多行标志），按非文本处理就打不了汉字（#867）。
 */
fun EditorInfo.wantsDirectInput(): Boolean {
    when (inputType and InputType.TYPE_MASK_CLASS) {
        InputType.TYPE_CLASS_TEXT -> {}
        InputType.TYPE_NULL -> return inputType == InputType.TYPE_NULL
        else -> return true
    }
    return when (inputType and InputType.TYPE_MASK_VARIATION) {
        InputType.TYPE_TEXT_VARIATION_PASSWORD,
        InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD,
        InputType.TYPE_TEXT_VARIATION_WEB_PASSWORD,
        InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS -> true
        else -> false
    }
}

/**
 * 网址框：标点原样上屏不转全角，组句中敲标点先原样落下敲的字母（`github.` 不换成候选再接句号）。
 */
fun EditorInfo.wantsPlainPunctuation(): Boolean =
    inputType and InputType.TYPE_MASK_CLASS == InputType.TYPE_CLASS_TEXT &&
        inputType and InputType.TYPE_MASK_VARIATION == InputType.TYPE_TEXT_VARIATION_URI

/**
 * 应用声明这个输入框不要个性化学习（浏览器的无痕标签页等，`IME_FLAG_NO_PERSONALIZED_LEARNING`）：按私密输入处理，
 * 同 PC 上应用声明的私密输入框。
 */
fun EditorInfo.wantsPrivacy(): Boolean = imeOptions and EditorInfo.IME_FLAG_NO_PERSONALIZED_LEARNING != 0

/** 电话、数字、日期时间输入框：弹出来直接是数字键盘；换到别的输入框时 [Composer.abandon] 回到字母页。 */
fun EditorInfo.wantsNumbers(): Boolean = when (inputType and InputType.TYPE_MASK_CLASS) {
    InputType.TYPE_CLASS_NUMBER, InputType.TYPE_CLASS_PHONE, InputType.TYPE_CLASS_DATETIME -> true
    else -> false
}
