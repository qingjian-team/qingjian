package app.qingjian.inputmethod.keyboard.layout

/** 键盘上的一个键；[weight] 是相对一个字母键的宽度。 */
sealed class Key(val label: String, val weight: Float = 1f) {
    /** 长按气泡里的项；空的不弹。 */
    open val choices: List<String> get() = emptyList()

    /** 长按不滑直接抬手选哪一项。 */
    open val initialChoice: Int get() = 0

    /**
     * 全键盘的字母键；[hint] 是键面角上的符号（首排数字、二三排常用符号）。
     * 双拼键盘另印这个键的翘舌声母 [initial] 与韵母 [finals]。
     */
    class Letter(
        val char: Char,
        val hint: String = "",
        val initial: String = "",
        val finals: String = "",
    ) : Key(char.toString()) {
        /** 长按气泡：大写、角标符号、小写，气泡对着键居中，不滑抬手出符号（与键面角标一致）。 */
        override val choices: List<String>
            get() = if (hint.isEmpty()) emptyList() else listOf(char.uppercase(), hint, char.toString())

        override val initialChoice: Int get() = 1
    }

    /** 九宫格的数字键：[digit] 进引擎，键面画 [letters]；`1` 是分词键。 */
    class Digit(val digit: Char, val letters: String) : Key(letters, KeyboardLayout.NINE_KEY_WIDTH) {
        /** 长按气泡里的项：小写字母、数字、大写字母（`abc2ABC`）；`1` 键是数字和几个常用符号。 */
        override val choices: List<String>
            get() = if (digit == '1') {
                listOf("1", "@", "-", "_", "/")
            } else {
                letters.lowercase().map(Char::toString) + digit.toString() + letters.map(Char::toString)
            }

        /** 长按数字键最常见的就是要这个数字。 */
        override val initialChoice: Int get() = choices.indexOf(digit.toString()).coerceAtLeast(0)
    }

    /** 注音大千键：键面画注音符号 [label]，进引擎的是它在大千键位上的 ASCII [code]（Core 的 `zhuyin::map_key`）。 */
    class Zhuyin(val code: Char, symbol: String, weight: Float) : Key(symbol, weight)

    /** 九宫格的「重输」：清空组句。 */
    data object Clear : Key("重输", 1.5f)

    /**
     * 直接上屏的标点、数字、符号。[punct] 不为空的是中文键盘的标点键：上屏前交 Core 按设置转全角或保持半角
     * （键上印的 [text] 只是样子，见 [KeyboardLayout.punct]）。
     */
    class Text(val text: String, weight: Float = 1f, val punct: Char? = null) : Key(text, weight)

    data object Shift : Key("", 1.5f)

    data object Backspace : Key("", 1.5f)

    /** 空格；数字键盘上的窄一些。 */
    class Space(weight: Float = 3f) : Key("青简", weight)

    class Enter(weight: Float = 1.75f) : Key("", weight)

    /** 打开分类符号面板（中文底排的「符」、英文符号页的「更多」）。 */
    class Symbols(label: String, weight: Float) : Key(label, weight)

    /** 切到常用符号页（英文底排的「!?#」）。 */
    class SymbolPage(label: String, weight: Float = 1f) : Key(label, weight)

    /** 切到数字键盘（底排的「123」）。 */
    class NumberPage(label: String, weight: Float = 1f) : Key(label, weight)

    /** 从符号页回到字母（中文「返回」、英文「ABC」）。 */
    class Letters(label: String, weight: Float = 1f) : Key(label, weight)

    /** 中 / 英切换。 */
    data object Language : Key("中", 1f)

    /** 功能键用深一档的底色。 */
    val functional: Boolean
        get() = this is Shift || this is Backspace || this is Symbols || this is SymbolPage ||
            this is NumberPage || this is Letters || this is Language || this is Clear ||
            (this is Digit && digit == '1')

    /** 按下时弹气泡的键。 */
    val poppable: Boolean get() = this is Letter || this is Zhuyin || (this is Text && weight == 1f)
}
