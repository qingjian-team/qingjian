package app.qingjian.inputmethod.theme

import app.qingjian.inputmethod.R

/**
 * 候选字号的几档：候选字与译文一起按 [scale] 缩放，对应 PC 的「候选字号」「译文字号」（手机上合成一项，按比例调）。
 * 只管候选条与展开的候选，键面上的字不跟着变。
 */
enum class TextSize(val key: String, val scale: Float, val label: Int) {
    SMALL("small", 0.88f, R.string.text_size_small),
    STANDARD("standard", 1f, R.string.text_size_standard),
    LARGE("large", 1.15f, R.string.text_size_large),
    LARGER("larger", 1.3f, R.string.text_size_larger);

    companion object {
        fun of(key: String?) = entries.firstOrNull { it.key == key } ?: STANDARD
    }
}
