package app.qingjian.inputmethod.core.frame

data class Sense(
    /** 词性缩写（n.）。 */
    val pos: String?,
    val text: String,
    val reading: String?,
    val fresh: Boolean,
)
