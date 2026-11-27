//! 按键按下时的修饰键快照，只拦截同一按键中途改变修饰键的重复。

package app.qingjian.inputmethod.service.input.physical

class Presses {
    private val modifiers = mutableMapOf<Pair<Int, Int>, Int>()

    fun down(device: Int, key: Int, state: Int, initial: Boolean) {
        if (initial) modifiers[device to key] = state
    }

    fun changed(device: Int, key: Int, state: Int): Boolean =
        modifiers[device to key]?.let { it != state } ?: false

    fun up(device: Int, key: Int) { modifiers.remove(device to key) }

    fun clear() { modifiers.clear() }
}
