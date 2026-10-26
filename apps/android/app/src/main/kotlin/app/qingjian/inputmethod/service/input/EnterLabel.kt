package app.qingjian.inputmethod.service.input

import android.view.inputmethod.EditorInfo

/**
 * 回车键上写什么：输入框声明了动作就写动作（发送、搜索、前往、下一项、上一项、完成），
 * 应用给了自己的文字就用它的；普通多行框、声明了「回车不做动作」的返回 null，键上画回车图标。
 * 与 [android.inputmethodservice.InputMethodService.sendDefaultEditorAction] 的判断一致：有字时回车执行动作，没字时换行。
 */
fun EditorInfo.enterLabel(): String? {
    if (imeOptions and EditorInfo.IME_FLAG_NO_ENTER_ACTION != 0) return null
    val action = imeOptions and EditorInfo.IME_MASK_ACTION
    if (action == EditorInfo.IME_ACTION_NONE || action == EditorInfo.IME_ACTION_UNSPECIFIED) return null
    actionLabel?.toString()?.takeIf { it.isNotBlank() && it.length <= MAX_LABEL }?.let { return it }
    return when (action) {
        EditorInfo.IME_ACTION_GO -> "前往"
        EditorInfo.IME_ACTION_SEARCH -> "搜索"
        EditorInfo.IME_ACTION_SEND -> "发送"
        EditorInfo.IME_ACTION_NEXT -> "下一项"
        EditorInfo.IME_ACTION_PREVIOUS -> "上一项"
        EditorInfo.IME_ACTION_DONE -> "完成"
        else -> null
    }
}

/** 应用给的动作文字太长就不用，键上放不下。 */
private const val MAX_LABEL = 4
