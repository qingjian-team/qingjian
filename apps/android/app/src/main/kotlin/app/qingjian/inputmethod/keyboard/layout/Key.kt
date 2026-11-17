package app.qingjian.inputmethod.keyboard.layout

/** 键盘上的一个键；[weight] 是相对一个字母键的宽度。 */
sealed class Key(val label: String, val weight: Float = 1f) {
    /** 长按气泡里的项；空的不弹。 */
    open val choices: List<String> get() = emptyList()

    /** 长按不滑直接抬手选哪一项。 */
    open val initialChoice: Int get() = 0

    /**
     * 在键上往上划做什么。
     *
     * 这是每个键**自己声明**的：判定（`KeyboardView.flickable`）与执行（`flick`）都只读它，
     * 不再按「是哪种键」四处分支。加一个新键、给某个键换一种上滑，只改这里一处。
     */
    open val flick: Flick get() = choices.getOrNull(initialChoice)?.let(Flick::Insert) ?: Flick.None

    /**
     * 在键上往下划做什么，缺省没有。
     *
     * 只有删除键用得上：往上划清空、往下划删整行、往左划删一个词，三个方向分开声明，互不覆盖。
     */
    open val flickDown: Flick get() = Flick.None

    /**
     * 在键上往左划做什么，缺省没有。
     *
     * 现在只有删除键声明：往左划删掉光标前的一个词（英文、数字按整词，汉字按一个字）。
     * 删除键往左划是不少手机输入法的习惯（#990 第 3 条）。
     */
    open val flickLeft: Flick get() = Flick.None

    /** 划出来的几种结果。 */
    sealed class Flick {
        /** 不能上滑：抬手出键本身。 */
        data object None : Flick()

        /** 上屏一段文字（字母键是角标，九宫格是数字，标点键是另一种宽度）。 */
        data class Insert(val text: String) : Flick()

        /** 当成按了另一个键（双拼占位键上滑出 Shift）。 */
        data class Press(val key: Key) : Flick()

        /** 清掉光标前的全部文字。 */
        data object ClearBefore : Flick()

        /** 删掉光标前的整行。 */
        data object DeleteLine : Flick()

        /** 删掉光标前的一个词（英文、数字按整词，汉字按一个字）。 */
        data object DeleteWord : Flick()
    }

    /**
     * 全键盘的字母键；[hint] 是键面角上的符号（首排数字、二三排常用符号）。
     * 双拼键盘另印这个键的翘舌声母 [initial] 与韵母 [finals]。
     * [flickShift] 的键往上一划是 Shift（微软双拼占了 Shift 位置的 `;`）。
     */
    class Letter(
        val char: Char,
        val hint: String = "",
        val initial: String = "",
        val finals: String = "",
        weight: Float = 1f,
        val flickShift: Boolean = false,
    ) : Key(char.toString(), weight) {
        /** 长按气泡：大写、角标符号、小写，气泡对着键居中，不滑抬手出符号（与键面角标一致；「长按出大写」开着时出大写）。 */
        override val choices: List<String>
            get() = if (hint.isEmpty()) emptyList() else listOf(char.uppercase(), hint, char.toString())

        override val initialChoice: Int get() = 1

        /** 上滑：占位键出 Shift，其余出角标符号（与长按不滑抬手那一项一致）。 */
        override val flick: Flick
            get() = when {
                flickShift -> Flick.Press(Shift)
                hint.isNotEmpty() -> Flick.Insert(hint)
                else -> Flick.None
            }
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
    class Text(val text: String, weight: Float = 1f, val punct: Char? = null) : Key(text, weight) {
        /**
         * 标点键上滑出它**另一种宽度**的写法：中文标点键上滑出半角 `,`，英文键盘上滑出全角 `，`。
         * 和字母键上滑出角标是同一套规则，逗号句号不必长按也能打另一半。
         */
        override val flick: Flick
            get() = KeyboardLayout.otherWidth(text)?.let(Flick::Insert) ?: Flick.None
    }

    data object Shift : Key("", 1.5f)

    data object Backspace : Key("", 1.5f) {
        /** 上滑：清掉光标前的全部文字（设置 `flickClear` 管着要不要这一步）。 */
        override val flick: Flick get() = Flick.ClearBefore

        /** 下滑：删掉光标前的整行（设置 `flickDeleteLine` 管着要不要这一步）。 */
        override val flickDown: Flick get() = Flick.DeleteLine

        /** 左滑：删掉光标前的一个词（设置 `flickDeleteWord` 管着要不要这一步）。 */
        override val flickLeft: Flick get() = Flick.DeleteWord
    }

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
