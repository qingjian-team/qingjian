package app.qingjian.inputmethod.keyboard

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.os.Handler
import android.os.Looper
import android.view.MotionEvent
import android.view.View
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.keyboard.layout.Key
import app.qingjian.inputmethod.keyboard.layout.KeySlot
import app.qingjian.inputmethod.keyboard.layout.KeyboardMetrics
import app.qingjian.inputmethod.keyboard.paint.KeyPainter
import app.qingjian.inputmethod.popup.ChoicePopup
import app.qingjian.inputmethod.settings.Settings
import app.qingjian.inputmethod.theme.Palette
import kotlin.math.abs
import kotlin.math.hypot

/**
 * 自绘键盘。多指各自跟踪（快打时前一指没抬后一指已落），抬起时出键；退格长按连发；
 * 九宫格的数字键长按弹气泡选字母 / 数字；开着「上滑输入」时在键上往上一划出长按的那一项（键面角标或数字）；
 * 全键盘拼音与英文键盘开着「滑行输入」时，落在字母键上的手指挪开够远就是滑行（[SwipeTrail]）。
 * 顶排的按键气泡要画到候选条上，父容器得关 clipChildren。
 */
@SuppressLint("ViewConstructor")
class KeyboardView(
    context: Context,
    palette: Palette,
    private val settings: Settings,
    private val listener: Listener,
) : View(context) {
    interface Listener {
        fun onKey(key: Key)

        /** 长按气泡里选了一项（字母、数字或符号）。 */
        fun onKeyChoice(text: String)

        /** 长按中英键选了第 [index] 种语言（[languageChoices] 的下标）。 */
        fun onLanguageChoice(index: Int)

        /** 长按空格左右滑：光标挪 [steps] 格，负数往左。 */
        fun onCursorMove(steps: Int)

        /** 九宫格拼音列里点了一项。 */
        fun onSideItem(item: String)

        /** 按住空格不动到点（开着「长按空格说话」）：开始说话，返回开没开起来；没开起来照旧进挪光标。 */
        fun onVoiceHold(): Boolean

        /** 按着空格说话时手指挪到了屏幕上 ([screenX], [screenY])：语音面板按它亮起撤回或发送。 */
        fun onVoiceDrag(screenX: Float, screenY: Float)

        /** 按着空格说话的手指抬起（松在哪块上由语音面板定）；[cancel] 是手指被系统收走。 */
        fun onVoiceRelease(cancel: Boolean)

        /** 滑完一笔：[input] 是轨迹与各字母键几何的 JSON，交 Rust 解码。 */
        fun onSwipe(input: String)

        /** 删除键按下（在它删字之前）：上划清空的撤销要记下这一刻光标前的文字。 */
        fun onBackspaceDown()

        /** 在删除键上往上一划（开着「上划删除键清空」）：清掉光标前的全部文字，组句中只清拼音。 */
        fun onClearBefore()
    }

    private val metrics = KeyboardMetrics(resources.displayMetrics.density, settings.heightPercent)
    private val painter = KeyPainter(palette, metrics, settings.keyTextSize.scale)
    private val side = SideColumn(metrics.density) { item ->
        tapFeedback()
        listener.onSideItem(item)
    }

    private var rows: List<List<Key>> = emptyList()
    private var slots: List<KeySlot> = emptyList()
    private val pressed = HashMap<Int, KeySlot>()

    private val timer = Handler(Looper.getMainLooper())
    private val repeatBackspace = object : Runnable {
        override fun run() {
            // 已经连删起来了：这一笔不再当上划清空，连删时手指往上偏一点不会把整段删掉
            if (flickSlot?.key == Key.Backspace) {
                flickBroken = true
                flickArmed = false
            }
            listener.onKey(Key.Backspace)
            timer.postDelayed(this, REPEAT_INTERVAL_MS)
        }
    }

    private val choicePopup = ChoicePopup(this, palette)
    private var choiceItems: List<String> = emptyList()

    /** 气泡是哪个键弹的：中英键选的是语言，不上屏。 */
    private var choiceKey: Key? = null
    private val screenLocation = IntArray(2)

    /** 在等长按的那根手指；气泡弹出后它的移动只用来选项。 */
    private var longPressPointer = -1

    /** 气泡对着的那一点（屏幕 x）；手指横向挪开 [TRACK_SLOP_DP] 才开始跟着选。 */
    private var choiceAnchorX = 0f
    private var choiceAnchorY = 0f
    private var choiceTracking = false
    private var longPressX = 0f
    private val longPress = Runnable { openChoices() }

    /**
     * 按着空格的那根手指；横着滑开就进挪光标（「空格滑动移动光标」开着时），按住不动到点进语音（关着「长按空格说话」或语音开不起来时进挪光标），
     * 这两种抬手都不出空格。
     */
    private var spacePointer = -1
    private var spaceAnchorX = 0f
    private var spaceAnchorY = 0f
    private var cursorMode = false
    private var voiceMode = false
    private val spaceHold = Runnable { onSpaceHold() }

    /**
     * 可能是上滑的那根手指与它落下的键（[Key.flick]，开着「上滑输入」时）；-1 是没在看。
     * 往上划过 [FLICK_DP] 且一路都在往上的扇形里（[FLICK_SLOPE]）就 [flickArmed]，抬手出上滑的字，不出键本身、也不算滑行；
     * 中途偏出扇形就 [flickBroken]，这一笔不再当上滑。
     */
    private var flickPointer = -1
    private var flickSlot: KeySlot? = null
    private var flickX = 0f
    private var flickY = 0f
    private var flickArmed = false
    private var flickBroken = false

    /** 这一套键位能不能滑行（全键盘拼音与英文键盘，服务按当前键盘设）；还要设置里开着「滑行输入」。 */
    var swipeLayout = false

    private val swipeOn: Boolean get() = swipeLayout && settings.swipeInput

    private val trail = SwipeTrail(palette, resources.displayMetrics.density)

    /** Shift 亮着字母画成大写，锁定时 Shift 键上另画一道横线。 */
    var shift: ShiftState
        get() = painter.shift
        set(value) {
            painter.shift = value
            invalidate()
        }

    /** 中英键上的当前语言与角上按一下切到的那种。 */
    var languageLabel: String
        get() = painter.languageLabel
        set(value) {
            painter.languageLabel = value
            invalidate()
        }

    var languageHint: String
        get() = painter.languageHint
        set(value) {
            painter.languageHint = value
            invalidate()
        }

    /** 长按中英键弹的语言（名字）与当前那个的下标。 */
    var languageChoices: List<String> = emptyList()
    var languageChoice = 0

    /** 中文模式；只影响中 / 英键上的字。 */
    var chinese: Boolean
        get() = painter.chinese
        set(value) {
            painter.chinese = value
            invalidate()
        }

    /** 回车键上的字，跟着输入框的动作走（见 `EditorInfo.enterLabel`）。 */
    var enterLabel: String?
        get() = painter.enterLabel
        set(value) {
            painter.enterLabel = value
            invalidate()
        }

    /** 100% 时键盘区的高度，拖动调高度时的位移按它换算成百分比。 */
    val baseHeight: Float get() = metrics.baseHeight

    init {
        setBackgroundColor(palette.board)
    }

    fun setRows(rows: List<List<Key>>, side: Boolean = false) {
        this.rows = rows
        this.side.enabled = side
        pressed.clear()
        relayout()
    }

    fun setSideItems(items: List<String>) {
        side.items = items
        invalidate()
    }

    /** 拖动调高度时逐帧调：键跟着拉伸，不重建视图（重建会断掉正在进行的拖动）。 */
    fun setHeightPercent(percent: Int) {
        metrics.setHeightPercent(percent)
        relayout()
        requestLayout()
    }

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        setMeasuredDimension(MeasureSpec.getSize(widthMeasureSpec), metrics.height)
    }

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) = relayout()

    private fun relayout() {
        slots = metrics.layout(rows, side.enabled, width)
        side.bounds = metrics.sideBounds(width)
        invalidate()
    }

    override fun onDraw(canvas: Canvas) {
        for (slot in slots) painter.drawKey(canvas, slot, down = pressed.containsValue(slot))
        if (side.enabled) painter.drawSide(canvas, side)
        val flicking = flickSlot?.takeIf { flickArmed }
        if (flicking == null) trail.draw(canvas)
        if (flicking != null) {
            // 上滑出的是哪个字放在气泡里；`;` 兼大写那个键划出的是 Shift，键本身亮着就够了
            val label = if (flicking.key == Key.Backspace) context.getString(R.string.flick_clear_bubble) else flicking.key.flick
            label?.let { painter.drawBubble(canvas, flicking, width, it) }
        } else if (settings.keyPopup) {
            pressed.values.filter { it.key.poppable && !choicePopup.showing }.forEach { painter.drawBubble(canvas, it, width) }
        }
    }

    @SuppressLint("ClickableViewAccessibility")
    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (side.onTouch(event, ::invalidate)) return true
        val index = event.actionIndex
        val pointer = event.getPointerId(index)
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN, MotionEvent.ACTION_POINTER_DOWN -> {
                // 滑行中别的手指落下不算；没滑开时第二根手指落下，说明在快打，这一笔不当滑行
                if (trail.swiping) return true
                trail.clear()
                val slot = slotAt(event.getX(index), event.getY(index)) ?: return true
                if (swipeOn && slot.key is Key.Letter && pressed.isEmpty()) {
                    trail.begin(pointer, event.getX(index), event.getY(index), slot.bounds.width())
                }
                pressed[pointer] = slot
                if (flickPointer < 0 && flickable(slot.key)) {
                    flickPointer = pointer
                    flickSlot = slot
                    flickX = event.getX(index)
                    flickY = event.getY(index)
                    flickArmed = false
                    flickBroken = false
                }
                tapFeedback()
                when {
                    slot.key == Key.Backspace -> {
                        if (settings.flickClear) listener.onBackspaceDown()
                        listener.onKey(Key.Backspace)
                        timer.postDelayed(repeatBackspace, REPEAT_DELAY_MS)
                    }

                    slot.key is Key.Space -> {
                        spacePointer = pointer
                        spaceAnchorX = event.getX(index)
                        spaceAnchorY = event.getY(index)
                        cursorMode = false
                        voiceMode = false
                        timer.postDelayed(spaceHold, if (settings.voiceHoldSpace) VOICE_HOLD_MS else LONG_PRESS_MS)
                    }

                    choicesOf(slot.key).isNotEmpty() && !choicePopup.showing -> {
                        timer.removeCallbacks(longPress)
                        longPressPointer = pointer
                        longPressX = event.getX(index)
                        timer.postDelayed(longPress, LONG_PRESS_MS)
                    }
                }
            }

            MotionEvent.ACTION_MOVE -> {
                trackFlick(event)
                trackSwipe(event)
                trackChoices(event)
                trackCursor(event)
            }

            MotionEvent.ACTION_UP, MotionEvent.ACTION_POINTER_UP -> {
                if (pointer == flickPointer) {
                    val flicked = flickSlot?.takeIf { flickArmed }
                    endFlick()
                    if (flicked != null) {
                        if (pointer == trail.pointer) trail.clear()
                        if (flicked.key == Key.Backspace) timer.removeCallbacks(repeatBackspace)
                        pressed.remove(pointer)
                        flick(flicked.key)
                        invalidate()
                        return true
                    }
                }
                if (pointer == trail.pointer) {
                    val swiped = trail.swiping
                    if (swiped) listener.onSwipe(trail.toJson(slots))
                    trail.clear()
                    if (swiped) {
                        pressed.remove(pointer)
                        invalidate()
                        return true
                    }
                }
                val slot = pressed.remove(pointer)
                val choosing = pointer == longPressPointer && choicePopup.showing
                val movedCursor = pointer == spacePointer && cursorMode
                val spoke = pointer == spacePointer && voiceMode
                if (pointer == spacePointer) {
                    timer.removeCallbacks(spaceHold)
                    spacePointer = -1
                    cursorMode = false
                    voiceMode = false
                }
                if (pointer == longPressPointer) {
                    timer.removeCallbacks(longPress)
                    longPressPointer = -1
                }
                when {
                    choosing -> choicePopup.finish()?.let { chosen ->
                        if (choiceKey == Key.Language) listener.onLanguageChoice(chosen) else listener.onKeyChoice(choiceItems[chosen])
                    }
                    slot?.key == Key.Backspace -> timer.removeCallbacks(repeatBackspace)
                    spoke -> listener.onVoiceRelease(cancel = false)
                    movedCursor -> Unit
                    slot != null -> listener.onKey(slot.key)
                }
            }

            MotionEvent.ACTION_CANCEL -> release()
        }
        invalidate()
        return true
    }

    /** 记着的那根手指挪动：点都记上（含两帧之间的历史点），刚挪够远就转成滑行，撤掉点按的按下态、气泡与长按。 */
    private fun trackSwipe(event: MotionEvent) {
        if (trail.pointer < 0 || choicePopup.showing) return
        val at = event.findPointerIndex(trail.pointer)
        if (at < 0) return
        var started = false
        for (h in 0 until event.historySize) started = trail.add(event.getHistoricalX(at, h), event.getHistoricalY(at, h)) || started
        started = trail.add(event.getX(at), event.getY(at)) || started
        if (!started) return
        pressed.remove(trail.pointer)
        if (longPressPointer == trail.pointer) {
            timer.removeCallbacks(longPress)
            longPressPointer = -1
        }
    }

    private fun flickable(key: Key) = (key as? Key.Letter)?.flickShift == true ||
        (settings.flickSymbols && key.flick != null) ||
        (settings.flickClear && key == Key.Backspace)

    /**
     * 上滑的那根手指挪动：只看方向不看落在哪一列，单手拇指划出的斜线也算（#882）。
     * 一离开按下的那一点就得在往上的扇形里，往上划够 [FLICK_DP] 算上滑；中途偏出扇形（横着走、往下、拐弯）就作废，
     * 交给滑行（[SwipeTrail]）或点按，所以跨键的滑行不会被当成上滑。
     * 刚往上走就撤掉等着的长按：慢慢往斜上划时长按先到点弹出气泡，手指再一歪就选上了气泡左边的大写。
     */
    private fun trackFlick(event: MotionEvent) {
        if (flickSlot == null || flickBroken) return
        val at = event.findPointerIndex(flickPointer)
        if (at < 0 || choicePopup.showing) return
        for (h in 0 until event.historySize) followFlick(event.getHistoricalX(at, h), event.getHistoricalY(at, h))
        followFlick(event.getX(at), event.getY(at))
    }

    private fun followFlick(x: Float, y: Float) {
        if (flickBroken) return
        val up = flickY - y
        val side = abs(x - flickX)
        // 按下附近的抖动不算方向
        if (hypot(x - flickX, up) < metrics.density * TRACK_SLOP_DP) return
        if (up <= 0f || side > up * FLICK_SLOPE) {
            flickBroken = true
            flickArmed = false
            return
        }
        if (longPressPointer == flickPointer) {
            timer.removeCallbacks(longPress)
            longPressPointer = -1
        }
        flickArmed = up > metrics.density * FLICK_DP
        // 删除键往上划够了：不再连删，抬手清空
        if (flickArmed && flickSlot?.key == Key.Backspace) timer.removeCallbacks(repeatBackspace)
    }

    private fun endFlick() {
        flickPointer = -1
        flickSlot = null
        flickArmed = false
        flickBroken = false
    }

    private fun flick(key: Key) = when {
        key == Key.Backspace -> listener.onClearBefore()
        (key as? Key.Letter)?.flickShift == true -> listener.onKey(Key.Shift)
        else -> key.flick?.let(listener::onKeyChoice)
    }

    /** 长按气泡里的项：中英键是语言名，其余是键自己的。 */
    private fun choicesOf(key: Key) = if (key == Key.Language) languageChoices else key.choices

    /** 长按到点：那根手指还按在这个键上就弹气泡，不滑直接抬手出 [Key.initialChoice]。字母键的气泡对着键居中。 */
    private fun openChoices() {
        val slot = pressed[longPressPointer] ?: return
        val key = slot.key
        choiceKey = key
        choiceItems = choicesOf(key)
        if (choiceItems.isEmpty()) return
        choiceTracking = false
        Haptics.hold(context)
        getLocationOnScreen(screenLocation)
        choicePopup.show(
            choiceItems.map { ChoicePopup.Item(it) },
            initial = when {
                key == Key.Language -> languageChoice
                key is Key.Letter && settings.longPressUpper -> 0
                else -> key.initialChoice
            },
            fingerScreenX = screenLocation[0] + slot.bounds.centerX(),
            bottom = slot.bounds.top,
            centred = key is Key.Letter,
            vertical = key == Key.Language,
        )
        choiceAnchorX = screenLocation[0] + longPressX
        choiceAnchorY = screenLocation[1] + slot.bounds.centerY()
        invalidate()
    }

    /** 气泡开着：那根手指横向（中英键的语言气泡是竖排，往上）滑到哪项选哪项，往下滑出一行算取消。 */
    private fun trackChoices(event: MotionEvent) {
        if (!choicePopup.showing) return
        val at = event.findPointerIndex(longPressPointer)
        val slot = pressed[longPressPointer]
        if (at < 0 || slot == null) return
        getLocationOnScreen(screenLocation)
        val x = screenLocation[0] + event.getX(at)
        val y = screenLocation[1] + event.getY(at)
        // 手指难免抖一点，挪开一段才开始选，不然没动就跳到别的项上
        val moved = if (choiceKey == Key.Language) abs(y - choiceAnchorY) else abs(x - choiceAnchorX)
        if (moved > metrics.density * TRACK_SLOP_DP) choiceTracking = true
        if (!choiceTracking) return
        choicePopup.track(
            x,
            cancelled = event.getY(at) > slot.bounds.bottom + metrics.rowHeight,
            fingerScreenY = y,
        )
    }

    /** 按住空格不动到点：开着「长按空格说话」先试语音，开不起来（还不能用、没权限）退回挪光标（挪光标也关着就当普通空格）。 */
    private fun onSpaceHold() {
        if (spacePointer < 0 || cursorMode || voiceMode) return
        if (settings.voiceHoldSpace && listener.onVoiceHold()) {
            voiceMode = true
            Haptics.hold(context)
            invalidate()
            return
        }
        if (settings.spaceCursor) enterCursorMode()
    }

    /** 按着空格说话：手指挪到哪儿交给语音面板（它盖在键盘上，按屏幕坐标对）。 */
    private fun trackVoice(event: MotionEvent) {
        val at = event.findPointerIndex(spacePointer)
        if (at < 0) return
        getLocationOnScreen(screenLocation)
        listener.onVoiceDrag(screenLocation[0] + event.getX(at), screenLocation[1] + event.getY(at))
    }

    /** 按住空格到点：进挪光标。 */
    private fun enterCursorMode() {
        if (spacePointer < 0 || cursorMode) return
        cursorMode = true
        Haptics.hold(context)
    }

    /** 按着空格横着滑：横向滑开一段（比竖向多）就进挪光标，之后每 [CURSOR_STEP_DP] 挪一格。 */
    private fun trackCursor(event: MotionEvent) {
        if (voiceMode) return trackVoice(event)
        val at = event.findPointerIndex(spacePointer)
        if (at < 0) return
        val dx = event.getX(at) - spaceAnchorX
        val step = metrics.density * CURSOR_STEP_DP
        val dy = event.getY(at) - spaceAnchorY
        if (!cursorMode && settings.spaceCursor && abs(dx) > step && abs(dx) > abs(dy)) enterCursorMode()
        if (!cursorMode) return
        val steps = (dx / step).toInt()
        if (steps == 0) return
        spaceAnchorX += steps * step
        listener.onCursorMove(steps)
    }

    private fun release() {
        pressed.clear()
        trail.clear()
        endFlick()
        timer.removeCallbacks(repeatBackspace)
        timer.removeCallbacks(longPress)
        timer.removeCallbacks(spaceHold)
        // 说着话的手指被系统收走（来电、键盘重建）：当取消
        if (voiceMode) listener.onVoiceRelease(cancel = true)
        spacePointer = -1
        cursorMode = false
        voiceMode = false
        longPressPointer = -1
        if (choicePopup.showing) choicePopup.finish()
    }

    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()
        release()
    }

    private fun tapFeedback() {
        Haptics.tap(context)
    }

    /** 键之间的缝也算最近那个键的，手指落在缝里不丢键。 */
    private fun slotAt(x: Float, y: Float): KeySlot? = slots.minByOrNull { slot ->
        val dx = maxOf(slot.bounds.left - x, 0f, x - slot.bounds.right)
        val dy = maxOf(slot.bounds.top - y, 0f, y - slot.bounds.bottom)
        dx * dx + dy * dy
    }

    private companion object {
        const val REPEAT_DELAY_MS = 400L
        const val REPEAT_INTERVAL_MS = 50L
        const val LONG_PRESS_MS = 350L

        /** 按住空格不动这么久进语音（开着「长按空格说话」时）。 */
        const val VOICE_HOLD_MS = 400L
        const val TRACK_SLOP_DP = 10f

        /** 往上划多远算上滑：点按时手指一般挪不到 10dp，再长短距离一划就够不着。 */
        const val FLICK_DP = 18f

        /**
         * 上滑的扇形：横向挪的不超过往上的这么多倍，约偏离竖直 50°（tan 50° ≈ 1.2），比横竖一样多（45°）再宽一点，
         * 给单手拇指的斜线留余量；再宽就和往斜上走的滑行分不开了。
         */
        const val FLICK_SLOPE = 1.2f
        const val CURSOR_STEP_DP = 14f
    }
}
