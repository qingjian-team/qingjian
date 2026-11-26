//! Android 物理按键与 Rust 会话共享的操作码。

package app.qingjian.inputmethod.service.input.physical

/** 物理键盘交给 Rust Session 的操作码，与 session/physical 对齐。 */
object Action {
    const val CHARACTER = 0

    const val CANDIDATE = 1

    const val RAW = 2

    const val BACKSPACE = 3

    const val DELETE = 4

    const val CLEAR = 5

    const val LEFT = 6

    const val RIGHT = 7

    const val HOME = 8

    const val END = 9

    const val PREVIOUS_PAGE = 10

    const val NEXT_PAGE = 11

    const val PREVIOUS = 12

    const val NEXT = 13
}
