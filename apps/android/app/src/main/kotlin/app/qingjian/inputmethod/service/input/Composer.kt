package app.qingjian.inputmethod.service.input

import android.os.Handler
import android.os.Looper
import android.view.KeyEvent
import android.view.inputmethod.InputConnection
import app.qingjian.inputmethod.core.Engine
import app.qingjian.inputmethod.core.InputOptions
import app.qingjian.inputmethod.core.frame.Candidate
import app.qingjian.inputmethod.core.frame.Frame
import app.qingjian.inputmethod.keyboard.layout.Key
import app.qingjian.inputmethod.keyboard.layout.KeyboardLayout
import app.qingjian.inputmethod.settings.Settings
import kotlin.math.abs

/**
 * 输入逻辑：把键盘的键翻成 Engine 操作，把帧落到输入框（组句文本）上，再让 [Host] 重画。
 * 不碰视图，也不管服务的生命周期。
 */
class Composer(private val engine: Engine, private val host: Host) {
    private val handler = Handler(Looper.getMainLooper())

    interface Host {
        val connection: InputConnection?

        /** Shift 亮着；出过一个大写字母后由这边熄掉。 */
        var shifted: Boolean

        fun sendKey(keyCode: Int)

        /** 执行输入框的缺省动作（搜索、发送、下一项）；没有缺省动作返回 false。 */
        fun performEditorAction(): Boolean

        fun showComposition(preedit: String, candidates: List<Candidate>)

        /** 键位表变了（中英、符号页、九宫格）。 */
        fun showKeyboard()

        fun showSideItems(pinyinChoices: List<String>)

        /** 打开分类符号面板。 */
        fun openSymbolPanel()

        /** 语言或中英键的另一边变了：记下来，中英键重画。 */
        fun onLanguageChanged()

        /** 一句话提示（删候选的结果）。 */
        fun showNotice(text: String)
    }

    /** 上一帧的 preedit，非空即在组句。 */
    private var preedit = ""

    private var pinyinChoices: List<String> = emptyList()

    /** 候选条上是补全（邮箱后缀），见 [refreshCompletions]。 */
    private var completing = false

    /** 这个输入框不过引擎，见 [wantsDirectInput]。 */
    var directInput = false

    /** 私密输入（不学习、不记统计与生词、不发云端），见 [wantsPrivacy]；引擎没装好时先记着，装好了在 [onEngineReady] 里补。 */
    var privateInput = false
        set(value) {
            field = value
            engine.setPrivate(value)
        }

    /** 现在用的语言。 */
    var language = Language.CHINESE
        private set

    /** 中英键按一下切到的另一种语言；与 [language] 是一对。 */
    var partner = Language.ENGLISH
        private set

    /** 长按挑语言时另一边固定是中文（缺省开）；关了就是「当前语言 ↔ 挑的那种」，见 [chooseLanguage]。 */
    var keepChinese = true

    val chinese: Boolean get() = language == Language.CHINESE

    /** 日语罗马音：敲的字母进 Core 的罗马音缓冲区，不经拼音引擎。 */
    val japanese: Boolean get() = language == Language.JAPANESE

    /** 键盘停在哪一页：字母、常用符号页（英文 !?#）、数字键盘（123）。 */
    enum class Page { LETTERS, SYMBOLS, NUMBERS }

    var page = Page.LETTERS
        private set

    var layout = Settings.Layout.QWERTY
        set(value) {
            if (field == value) return
            field = value
            applyScheme()
        }

    /** 候选出繁体（功能面板的「繁体」）；引擎没装好时先记着，装好了在 [onEngineReady] 里补上。 */
    var traditional = false
        set(value) {
            if (field == value) return
            field = value
            if (engine.ready) apply(engine.setTraditional(value))
        }

    /** 九宫格只管中文；英文模式退回全键盘。 */
    val nineKey: Boolean get() = layout == Settings.Layout.NINE_KEY && chinese

    /** 注音（大千）同样只管中文，英文退回全键盘。 */
    val zhuyin: Boolean get() = layout == Settings.Layout.ZHUYIN && chinese

    /** 双拼、五笔也只管中文；英文退回全键盘、英文模式。 */
    val shuangpin: Boolean get() = layout == Settings.Layout.SHUANGPIN && chinese

    val wubi: Boolean get() = layout == Settings.Layout.WUBI && chinese

    /** 双拼方案与 preedit 写法、五笔混输（键盘设置里的，同 PC 的 `[general]` 那几项）。 */
    var shuangpinScheme = Settings.DEFAULT_SHUANGPIN
    var shuangpinRawPreedit = false
    var wubiMixed = false

    /** 模糊音、中文标点全半角、打拼音时给不给英文词（键盘设置里的，交给 Core）；引擎没装好时先记着。 */
    var options = InputOptions()
        set(value) {
            if (field == value) return
            field = value
            if (engine.ready) apply(engine.setOptions(value))
        }

    /** 英文键盘也组句给英文词候选；关了字母直接上屏。 */
    var englishCandidates = true

    /** 敲的字母进组句：中文、日语总是进，英文看 [englishCandidates]。 */
    private val composesLetters: Boolean get() = !directInput && (chinese || japanese || englishCandidates)

    private val composing: Boolean get() = preedit.isNotEmpty()

    /** 组句处在算式（`v1+2`）或码点里，见 [Frame.expression]。 */
    private var expression = false

    /** 上一帧的候选。 */
    private var candidates: List<Candidate> = emptyList()

    /** 滑完一笔（[input] 是轨迹与键位的 JSON）：同点字母，组句时解出的字母进组句，字母直接上屏时上屏解出的整串（Shift 亮着首字母大写）。 */
    fun onSwipe(input: String) {
        if (composing || (engine.ready && composesLetters && !host.shifted)) {
            apply(engine.swipe(input))
            return
        }
        val letters = engine.decodeSwipe(input)
        if (letters.isEmpty()) return
        commit(if (host.shifted) letters.replaceFirstChar { it.uppercase() } else letters)
        host.shifted = false
    }

    fun onKey(key: Key) {
        when (key) {
            is Key.Letter -> when {
                composing || (engine.ready && composesLetters && !host.shifted) ->
                    apply(engine.push(key.char.code))

                else -> {
                    commit(if (host.shifted) key.label.uppercase() else key.label)
                    host.shifted = false
                }
            }

            is Key.Zhuyin ->
                if (engine.ready && !directInput) apply(engine.push(key.code.code)) else commit(key.label)

            is Key.Digit -> when {
                !engine.ready -> Unit
                key.digit != SEPARATOR_DIGIT -> apply(engine.push(key.digit.code))
                // 分词键只在组句中有意义
                composing -> apply(engine.push('\''.code))
            }

            is Key.Text -> {
                // 日语里的长音：罗马音的 `-`，进缓冲区转成「ー」
                if (japanese && engine.ready && !directInput && key.text in LONG_VOWEL) {
                    apply(engine.push('-'.code))
                    return
                }
                if (composing && expression) {
                    onExpressionText(key.text)
                    return
                }
                when {
                    !composing -> Unit
                    // 敲邮箱：@ 前的字母是用户名，原样落下，不顶首选
                    key.text == "@" && !zhuyin -> apply(engine.commitRaw())
                    else -> selectFirst()
                }
                // 中文标点键：Core 按设置转全角（引号成对、数字后的点保持半角）或原样
                val punct = key.punct
                commit(if (punct != null && chinese && !directInput) engine.punctuate(punct) else key.text)
            }

            Key.Clear -> if (composing) apply(engine.clear())

            Key.Shift -> if (!composing) host.shifted = !host.shifted

            Key.Backspace ->
                if (composing) apply(engine.backspace()) else host.sendKey(KeyEvent.KEYCODE_DEL)

            is Key.Space -> when {
                !composing -> commit(" ")
                chinese || japanese -> apply(engine.space())
                // 英文模式：敲的字母原样上屏，空格照给
                else -> {
                    apply(engine.commitRaw())
                    commit(" ")
                }
            }

            is Key.Enter -> when {
                composing -> settle()
                !host.performEditorAction() -> host.sendKey(KeyEvent.KEYCODE_ENTER)
            }

            is Key.Symbols -> {
                if (composing) selectFirst()
                host.openSymbolPanel()
            }

            is Key.SymbolPage, is Key.NumberPage, is Key.Letters -> {
                // 算式里换去数字键盘接着打，不顶首选
                if (composing && !expression) selectFirst()
                page = when (key) {
                    is Key.SymbolPage -> Page.SYMBOLS
                    is Key.NumberPage -> Page.NUMBERS
                    else -> Page.LETTERS
                }
                host.showKeyboard()
            }

            Key.Language -> {
                val from = language
                switchTo(partner)
                partner = from
                host.onLanguageChanged()
            }
        }
    }

    /** 上次用的语言与另一边（从设置里读回来）；引擎的模式在 [onEngineReady] 里补。 */
    fun restoreLanguages(language: Language, partner: Language) {
        this.language = language
        this.partner = when {
            partner != language -> partner
            language == Language.CHINESE -> Language.ENGLISH
            else -> Language.CHINESE
        }
    }

    /**
     * 长按中英键或键盘选择里挑了 [target]：切过去，并把中英键的另一边定下来。[keepChinese] 开着另一边总有中文
     * （挑中文时另一边是切走前那种）；关着另一边就是切走前的语言，英 ↔ 日 这种也行。
     */
    fun chooseLanguage(target: Language) {
        if (target == language) return
        val from = language
        partner = if (keepChinese && target != Language.CHINESE && from != Language.CHINESE) Language.CHINESE else from
        switchTo(target)
        host.onLanguageChanged()
    }

    /** 切到 [target]：离开日语先关罗马音缓冲区，进英文 / 中文换引擎的模式；组句中的输入由会话先落下。 */
    private fun switchTo(target: Language) {
        if (target == language) return
        if (composing && zhuyin) selectFirst()
        val wasJapanese = japanese
        language = target
        if (engine.ready) {
            if (wasJapanese) apply(engine.setJapanese(false))
            applyLanguage()
        }
        host.showKeyboard()
    }

    /** 引擎按 [language] 设好模式。 */
    private fun applyLanguage() {
        if (japanese) {
            apply(engine.setJapanese(true))
        } else {
            apply(engine.setEnglishMode(!chinese))
            applyScheme()
        }
    }

    /** 符号 / 表情面板里点的一格：组句中先顶首选再上屏。 */
    fun commitPanelText(text: String) {
        if (composing) selectFirst()
        commit(text)
    }

    /** 开始说话前：组句落下（同回车），识别出的字接在后面。 */
    fun prepareVoice() {
        if (composing) settle()
    }

    /** 识别出的字上屏：过引擎（繁体输出、输入历史与统计）；不过引擎的输入框与引擎没装好时原样上屏。 */
    fun commitVoice(text: String) {
        if (engine.ready && !directInput) apply(engine.commitVoice(text)) else commit(text)
    }

    fun select(index: Int) = apply(engine.select(index))

    fun selectTranslation(index: Int, sense: Int) = apply(engine.selectTranslation(index, sense))

    /** 拼音列：组句中点的是拼音（定下这个音节），平时点的是标点。 */
    fun onSideItem(item: String) {
        if (item in pinyinChoices) apply(engine.pin(item)) else onKey(KeyboardLayout.sideKey(item))
    }

    /** 长按候选选了「删除」：用户词整删、词库词清学习记录，结果用一句话提示。 */
    fun forget(index: Int) = apply(engine.forget(index))

    /**
     * 长按气泡里选的字符。全键盘选小写 = 照常敲这个字母；九宫格组句中选小写 = 这一位就是这个字母
     * （进缓冲区，和拼音列定音节一个道理）。其余（数字、大写、符号）先顶首选再上屏。
     */
    fun onKeyChoice(text: String) {
        val letter = text.singleOrNull()?.takeIf { it in 'a'..'z' }
        when {
            // 全键盘长按选小写 = 照常敲这个字母
            letter != null && !nineKey -> onKey(Key.Letter(letter))
            composing && letter != null && engine.ready -> apply(engine.push(letter.code))
            else -> onKey(Key.Text(text))
        }
    }

    /**
     * 算式里按的符号键：数字与运算符进缓冲区（× ÷ 换成 Core 认的 * /）；`=` 上屏「算式=结果」那条，没有就上屏首选；
     * 别的符号先顶首选再上屏。
     */
    private fun onExpressionText(text: String) {
        val c = text.singleOrNull()?.let(engine::expressionChar)
        when {
            c != null -> apply(engine.push(c.code))
            c == '=' -> {
                val index = candidates.indexOfFirst { '=' in it.text }
                if (index >= 0) select(index) else selectFirst()
            }

            else -> {
                selectFirst()
                commit(text)
            }
        }
    }

    /** 长按空格左右滑：不在组句时按方向键挪光标；组句中先把敲的字母原样落下再挪。 */
    fun moveCursor(steps: Int) {
        if (steps == 0) return
        if (composing) settle()
        val code = if (steps < 0) KeyEvent.KEYCODE_DPAD_LEFT else KeyEvent.KEYCODE_DPAD_RIGHT
        repeat(abs(steps)) { host.sendKey(code) }
    }

    /**
     * 光标停下（上屏、删字、挪光标之后）且不在组句：按光标前的文字问 Core 有没有补全（「用户名@」后的邮箱后缀），
     * 有就放在候选条上，选中走 [select]。只显示、不写输入框，所以不会再触发光标变化。
     */
    fun refreshCompletions() {
        if (composing || !engine.ready) return
        val before = host.connection?.getTextBeforeCursor(COMPLETION_CONTEXT, 0)?.toString().orEmpty()
        if (!completing && '@' !in before && before.lastOrNull()?.let(engine::mayEndCalculation) != true) return
        val frame = engine.complete(before)
        completing = frame.candidates.isNotEmpty()
        host.showComposition("", frame.candidates)
    }

    /** 组句中光标被点到别处：敲的字母原样落下，不留带 ' 的显示形式。 */
    fun onCursorMovedAway() {
        if (composing) settle()
    }

    /** 输入框换了或键盘收起：组句不带走。 */
    fun abandon() {
        preedit = ""
        pinyinChoices = emptyList()
        expression = false
        candidates = emptyList()
        completing = false
        engine.reset()
        host.shifted = false
        host.showSideItems(pinyinChoices)
        host.showComposition("", emptyList())
        if (page != Page.LETTERS) {
            page = Page.LETTERS
            host.showKeyboard()
        }
    }

    /** 引擎刚装好：补上输入方案与繁体开关。 */
    fun onEngineReady() {
        engine.setPrivate(privateInput)
        applyScheme()
        if (!chinese) applyLanguage()
        if (traditional) apply(engine.setTraditional(true))
        apply(engine.setOptions(options))
    }

    /** 引擎的输入方案跟着键盘走；引擎装好、换布局、切中英时调。 */
    fun applyScheme() {
        if (!engine.ready) return
        apply(engine.setNineKey(nineKey))
        apply(engine.setZhuyin(zhuyin))
        apply(engine.setShuangpin(if (shuangpin) shuangpinScheme else "", shuangpinRawPreedit))
        apply(engine.setWubi(wubi, wubiMixed))
    }

    /**
     * 回车、挪光标时把组句落下：拼音原样落下敲的字母；注音缓冲区里是大千键位的 ASCII，落下没意义，上屏首选（与桌面端回车一致）。
     */
    private fun settle() = if (zhuyin) selectFirst() else apply(engine.commitRaw())

    /** 空格 / 标点顶首选；没有候选（切不动）就原样上屏。 */
    private fun selectFirst() {
        var frame = engine.select(0)
        if (frame.commit.isEmpty() && frame.preedit.isNotEmpty()) frame = engine.commitRaw()
        apply(frame)
    }

    private fun apply(frame: Frame) {
        val connection = host.connection ?: return
        // 一段组句开头：组合文字放进去之前读光标前后文，给云联想
        val surrounding = if (engine.predicting && preedit.isEmpty() && frame.preedit.isNotEmpty()) {
            (connection.getTextBeforeCursor(SURROUNDING_BEFORE, 0)?.toString().orEmpty()) to
                (connection.getTextAfterCursor(SURROUNDING_AFTER, 0)?.toString().orEmpty())
        } else {
            null
        }
        connection.beginBatchEdit()
        if (frame.replace > 0) connection.deleteSurroundingText(frame.replace, 0)
        if (frame.commit.isNotEmpty()) connection.commitText(frame.commit, 1)
        if (frame.preedit.isNotEmpty()) {
            connection.setComposingText(frame.preedit, 1)
        } else if (frame.commit.isEmpty()) {
            connection.setComposingText("", 1)
            connection.finishComposingText()
        }
        connection.endBatchEdit()
        preedit = frame.preedit
        pinyinChoices = frame.pinyinChoices
        expression = frame.expression
        candidates = frame.candidates
        completing = false
        host.showSideItems(pinyinChoices)
        host.showComposition(frame.preedit, frame.candidates)
        if (frame.notice.isNotEmpty()) host.showNotice(frame.notice)
        surrounding?.let { (before, after) -> engine.setSurrounding(before, after) }
        handler.removeCallbacks(pollPrediction)
        if (engine.predicting && composing) handler.postDelayed(pollPrediction, PREDICTION_POLL_MS)
    }

    /** 组句中隔一会儿取一次云联想结果；到了就插进候选重画，组句结束不再取。 */
    private val pollPrediction: Runnable = Runnable {
        if (!composing) return@Runnable
        val frame = engine.pollPrediction()
        if (frame != null && frame.preedit == preedit) {
            candidates = frame.candidates
            host.showComposition(frame.preedit, frame.candidates)
        }
        if (engine.predicting) handler.postDelayed(pollPrediction, PREDICTION_POLL_MS)
    }

    private fun commit(text: String) {
        host.connection?.commitText(text, 1)
    }

    private companion object {
        const val SEPARATOR_DIGIT = '1'

        /** 云联想给的光标前后文：Engine 会按观察窗口再裁（前 64、后 32）。 */
        const val SURROUNDING_BEFORE = 64
        const val SURROUNDING_AFTER = 32

        /** 组句中取云联想结果的间隔。 */
        const val PREDICTION_POLL_MS = 150L

        /** 补全看光标前多少个字：用户名加域名前缀够了。 */
        const val COMPLETION_CONTEXT = 80

        /** 日语里当长音敲的键。 */
        val LONG_VOWEL = setOf("-", "ー")
    }
}
