package app.qingjian.inputmethod.panel.editor

import android.view.inputmethod.InputConnection
import app.qingjian.inputmethod.translate.TranslateBar

/**
 * 浮在键盘上方、接管打字的编辑条（常用语编辑框 [PhraseEditor]、快捷翻译 [TranslateBar]）：
 * 开着时输入法服务把 Composer 的输入连接换成它的 [connection]，键盘照常打字，字进编辑条而不是外面的输入框。
 */
interface FloatingEditor {
    val connection: InputConnection

    /** Composer 不在组句时发来的键（删除、回车）。 */
    fun onKey(keyCode: Int)

    /** 只收字母：键盘换成英文全键盘（九宫格、注音打不出字母）。 */
    val lettersOnly: Boolean get() = false

    /** 回车键上写的字；null 用外面输入框的。 */
    val enterLabel: String? get() = null
}
