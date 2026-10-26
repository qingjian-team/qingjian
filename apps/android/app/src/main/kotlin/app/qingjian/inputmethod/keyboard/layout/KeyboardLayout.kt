package app.qingjian.inputmethod.keyboard.layout

import app.qingjian.inputmethod.core.store.ShuangpinKeycap

/** 键位表：每行的键按 weight 分行宽，不满 10 个单位的行两边留白居中。 */
object KeyboardLayout {
    const val UNITS_PER_ROW = 10f

    /** 九宫格左侧拼音列的宽度；右列功能键是 1.5，中间三个数字键平分剩下的。 */
    const val SIDE_WIDTH = 1.5f
    const val NINE_KEY_WIDTH = (UNITS_PER_ROW - SIDE_WIDTH - 1.5f) / 3

    /** 九宫格里拼音列占的行数（前三行），这几行的键从拼音列右边排起。 */
    const val SIDE_ROWS = 3

    /** 中文标点键：全角的样子与对应的半角字符；半角标点设置下键上印半角。 */
    private val PUNCTUATION = mapOf('，' to ',', '。' to '.', '？' to '?', '！' to '!', '：' to ':')

    /** 中文键盘的一个标点键（[full] 是全角的样子），见 [Key.Text.punct]。 */
    fun punct(full: Char, fullWidth: Boolean, weight: Float = 1f): Key.Text {
        val half = PUNCTUATION.getValue(full)
        return Key.Text(if (fullWidth) full.toString() else half.toString(), weight, punct = half)
    }

    /** 不在组句时拼音列里放的标点；、和 … 没有对应的半角键，总是全角。 */
    fun sidePunctuation(fullWidth: Boolean): List<String> =
        listOf('，', '。', '？', '！').map { punct(it, fullWidth).text } + listOf("、", punct('：', fullWidth).text, "…")

    /** 拼音列里点的标点按哪个键算：认得的走 Core 的标点转换，其余原样。 */
    fun sideKey(item: String): Key.Text {
        val full = PUNCTUATION.entries.firstOrNull { (full, half) -> item == full.toString() || item == half.toString() }?.key
        return Key.Text(item, punct = full?.let(PUNCTUATION::getValue))
    }

    /** 字母键角标：首排数字，二排常用符号（中文把 H J 换成中文引号），三排括号与标点。 */
    private const val TOP_HINTS = "1234567890"
    private const val MIDDLE_HINTS_EN = "~!@#%'&*?"
    private const val MIDDLE_HINTS_ZH = "~!@#%“”*?"
    private const val BOTTOM_HINTS = "()-_:;/"

    /** 日语罗马音的角标：二排换成 ¥ 与日文括号「」，三排是引号与句读。 */
    private const val MIDDLE_HINTS_JA = "@#¥_&「」()"
    private const val BOTTOM_HINTS_JA = "*\"':;!?"

    private fun letters(row: String, hints: String) = row.zip(hints) { char, hint -> Key.Letter(char, hint.toString()) }

    /** 底排：两个切换键、逗号、空格、句号、中英、回车。中文的逗号句号走 Core 的标点转换（[fullWidth] 只管键上印什么）。 */
    private fun bottomRow(first: Key, second: Key, chinese: Boolean, fullWidth: Boolean = true) = listOf(
        first,
        second,
        if (chinese) punct('，', fullWidth) else Key.Text(","),
        Key.Space(),
        if (chinese) punct('。', fullWidth) else Key.Text("."),
        Key.Language,
        Key.Enter(),
    )

    /** 中文底排：「符」进分类符号面板；英文底排：「!?#」进常用符号页；「123」都进数字键盘。 */
    private fun lettersBottom(chinese: Boolean, fullWidth: Boolean = true) = if (chinese) {
        bottomRow(Key.Symbols("符", 1.25f), Key.NumberPage("123"), chinese = true, fullWidth)
    } else {
        bottomRow(Key.SymbolPage("!?#", 1.25f), Key.NumberPage("123"), chinese = false)
    }

    fun qwerty(chinese: Boolean, japanese: Boolean = false, fullWidth: Boolean = true): List<List<Key>> = if (japanese) {
        japaneseRows()
    } else {
        listOf(
            letters("qwertyuiop", TOP_HINTS),
            letters("asdfghjkl", if (chinese) MIDDLE_HINTS_ZH else MIDDLE_HINTS_EN),
            listOf(Key.Shift) + letters("zxcvbnm", BOTTOM_HINTS) + Key.Backspace,
            lettersBottom(chinese, fullWidth),
        )
    }

    /**
     * 日语罗马音：二排末尾补一个长音「ー」（敲罗马音的 `-`）占满十格；
     * 底排与拼音同宽，标点是「、」「。」。
     */
    private fun japaneseRows() = listOf(
        letters("qwertyuiop", TOP_HINTS),
        letters("asdfghjkl", MIDDLE_HINTS_JA) + Key.Text("ー"),
        listOf(Key.Shift) + letters("zxcvbnm", BOTTOM_HINTS_JA) + Key.Backspace,
        listOf(Key.SymbolPage("!?#", 1.25f), Key.NumberPage("123"), Key.Text("、"), Key.Space(), Key.Text("。"), Key.Language, Key.Enter()),
    )

    /**
     * 双拼：键位同拼音全键盘，键上另印当前方案的声韵母（从 Core 取，见 [Shuangpin][app.qingjian.inputmethod.core.store.Shuangpin]）；
     * 用 `;` 当韵母的方案二排末尾补一个 `;` 键。
     */
    fun shuangpin(keycaps: Map<Char, ShuangpinKeycap>, fullWidth: Boolean = true): List<List<Key>> {
        fun row(chars: String, hints: String) = chars.mapIndexed { i, char ->
            val keycap = keycaps[char]
            Key.Letter(char, hints.getOrNull(i)?.toString().orEmpty(), keycap?.initial.orEmpty(), keycap?.finals.orEmpty())
        }
        val middle = if (';' in keycaps) "asdfghjkl;" else "asdfghjkl"
        return listOf(
            row("qwertyuiop", TOP_HINTS),
            row(middle, MIDDLE_HINTS_ZH),
            listOf(Key.Shift) + row("zxcvbnm", BOTTOM_HINTS) + Key.Backspace,
            lettersBottom(chinese = true, fullWidth),
        )
    }

    fun nineKey(fullWidth: Boolean = true): List<List<Key>> = listOf(
        listOf(Key.Digit('1', "分词"), Key.Digit('2', "ABC"), Key.Digit('3', "DEF"), Key.Backspace),
        listOf(Key.Digit('4', "GHI"), Key.Digit('5', "JKL"), Key.Digit('6', "MNO"), Key.Clear),
        listOf(Key.Digit('7', "PQRS"), Key.Digit('8', "TUV"), Key.Digit('9', "WXYZ"), Key.Text("0", 1.5f)),
        lettersBottom(chinese = true, fullWidth),
    )

    /** 大千键位：注音符号与它在电脑键盘上的键，一行行对齐（同 Core 的 `zhuyin::map_key`）。 */
    private val ZHUYIN_ROWS = listOf(
        "1234567890-" to "ㄅㄉˇˋㄓˊ˙ㄚㄞㄢㄦ",
        "qwertyuiop" to "ㄆㄊㄍㄐㄔㄗㄧㄛㄟㄣ",
        "asdfghjkl;" to "ㄇㄋㄎㄑㄕㄘㄨㄜㄠㄤ",
        "zxcvbnm,./" to "ㄈㄌㄏㄒㄖㄙㄩㄝㄡㄥ",
    )

    /**
     * 注音大千：四排注音照电脑大千键位排，首排 11 个键（含声调）略窄，
     * 末排右边是删除；底排与拼音全键盘一样。空格是一声（还缺声调时），见 Session::space。
     */
    fun zhuyin(fullWidth: Boolean = true): List<List<Key>> {
        fun row(codes: String, symbols: String, weight: Float) =
            codes.zip(symbols) { code, symbol -> Key.Zhuyin(code, symbol.toString(), weight) }
        val (top, second, third, bottom) = ZHUYIN_ROWS
        return listOf(
            row(top.first, top.second, UNITS_PER_ROW / top.first.length),
            row(second.first, second.second, 1f),
            row(third.first, third.second, 1f),
            row(bottom.first, bottom.second, ZHUYIN_BOTTOM_WIDTH) + Key.Backspace,
            lettersBottom(chinese = true, fullWidth),
        )
    }

    /** 注音末排十个键的宽，给删除留出位置。 */
    private const val ZHUYIN_BOTTOM_WIDTH = 0.85f

    /** 数字键盘左列的符号，可以上下拖（与九宫格拼音列同一个控件）。 */
    val NUMBER_SIDE = listOf("%", "/", "-", "+", "=", "*", ",", ":", "(", ")", "#", "\$")

    /**
     * 数字键盘：左列符号，中间 3×3 数字，右列 删除 / . / @；底排 符 / 返回 / 0 / 空格 / 回车，
     * 与上面各列对齐，所以复用九宫格的列宽。
     */
    fun numbers(): List<List<Key>> {
        fun digits(row: String) = row.map { Key.Text(it.toString(), NINE_KEY_WIDTH) }
        return listOf(
            digits("123") + Key.Backspace,
            digits("456") + Key.Text(".", SIDE_WIDTH),
            digits("789") + Key.Text("@", SIDE_WIDTH),
            listOf(
                Key.Symbols("符", SIDE_WIDTH),
                Key.Letters("返回", NINE_KEY_WIDTH),
                Key.Text("0", NINE_KEY_WIDTH),
                Key.Space(NINE_KEY_WIDTH),
                Key.Enter(SIDE_WIDTH),
            ),
        )
    }

    /** 常用符号页。英文：数字、常用符号，「更多」进完整符号面板，「ABC」回字母。 */
    fun symbols(chinese: Boolean, fullWidth: Boolean = true): List<List<Key>> = if (chinese) {
        listOf(
            "1234567890".map { Key.Text(it.toString()) },
            "、；：？！（）《》～".map { Key.Text(it.toString()) },
            listOf<Key>(Key.Text("…", 1.5f)) + "“”‘’【】—".map { Key.Text(it.toString()) } + Key.Backspace,
            bottomRow(Key.Symbols("符", 1.25f), Key.Letters("返回"), chinese = true, fullWidth),
        )
    } else {
        listOf(
            "1234567890".map { Key.Text(it.toString()) },
            "@#\$_&-+()/".map { Key.Text(it.toString()) },
            listOf<Key>(Key.Symbols("更多", 1.5f)) + "*\"':;!?".map { Key.Text(it.toString()) } + Key.Backspace,
            bottomRow(Key.Letters("ABC", 1.25f), Key.NumberPage("123"), chinese = false),
        )
    }
}
