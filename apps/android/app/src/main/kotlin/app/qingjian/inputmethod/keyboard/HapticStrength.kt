package app.qingjian.inputmethod.keyboard

import app.qingjian.inputmethod.R

/** 按键震动的强度（#775），键盘设置里选；[STANDARD] 是原来的手感，关震动另有开关。 */
enum class HapticStrength(val key: String, val label: Int) {
    LIGHT("light", R.string.haptic_light),
    STANDARD("standard", R.string.haptic_standard),
    STRONG("strong", R.string.haptic_strong),
    ;

    companion object {
        fun of(key: String?) = entries.firstOrNull { it.key == key } ?: STANDARD
    }
}
