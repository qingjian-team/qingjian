//! 记住引擎要求的拼音光标位置，区分本方选区回调与应用内点击。

package app.qingjian.inputmethod.service.input.physical

class Caret {
    private var text = ""

    private var offset = 0

    fun expect(preedit: String, cursor: Int) {
        text = preedit
        offset = preedit.offsetByCodePoints(0, cursor.coerceIn(0, preedit.codePointCount(0, preedit.length)))
    }

    /** 应用改了组字内折叠光标时返回Core需要的字符下标；本方回调和组字外变化不处理。 */
    fun changed(preedit: String, start: Int, end: Int, composingStart: Int, composingEnd: Int): Int? {
        if (preedit.isEmpty() || composingStart < 0 || composingEnd - composingStart != preedit.length ||
            start != end || start !in composingStart..composingEnd
        ) return null
        val actual = start - composingStart
        if (preedit == text && actual == offset) return null
        return preedit.codePointCount(0, actual)
    }
}
