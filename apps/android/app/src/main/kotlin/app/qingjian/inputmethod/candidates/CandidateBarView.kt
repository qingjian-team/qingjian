package app.qingjian.inputmethod.candidates

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.view.GestureDetector
import android.view.MotionEvent
import android.view.View
import android.widget.OverScroller
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.core.frame.Candidate
import app.qingjian.inputmethod.keyboard.Haptics
import app.qingjian.inputmethod.panel.base.PanelIcons
import app.qingjian.inputmethod.popup.ChoicePopup
import app.qingjian.inputmethod.theme.Palette

/**
 * 键盘上方那一条，三种样子：组句中是候选（[CandidateStrip]，位图来自 [source]，横向滑动、长按弹译词气泡），
 * 平时是工具栏（[Toolbar]，中间那排可横滑），调高度时是拖动手柄（[ResizeHandle]）。
 */
@SuppressLint("ViewConstructor")
class CandidateBarView(
    context: Context,
    private val palette: Palette,
    /** 候选字号档的倍数：调大时条跟着高，候选不被截。 */
    textScale: Float,
    private val source: StripSource,
    private val listener: Listener,
) : View(context) {
    interface Listener {
        fun onSelect(index: Int)

        /** 长按气泡里选了译词：上屏第 [index] 个候选的第 [sense] 条译文。 */
        fun onSelectTranslation(index: Int, sense: Int)

        /** 长按气泡里选了「删除」：删第 [index] 个候选（用户词、学习记录）。 */
        fun onForget(index: Int)

        /** 工具栏上点了一个图标；调高度在这里自己处理，不会传出去。 */
        fun onToolbar(action: Toolbar.Action)

        /** 拖动调高度：[deltaPx] 是手指相对按下处往上走的距离，[done] 表示手指抬起。 */
        fun onResizeDrag(deltaPx: Float, done: Boolean)

        /** 点了候选右端的 ⌄：展开全部候选（[CandidateGrid]）。 */
        fun onExpand()
    }

    private val density = resources.displayMetrics.density
    val barHeight = 62f * density * maxOf(1f, textScale)
    private val strip = CandidateStrip(density, barHeight, textScale)
    private val toolbar = Toolbar(context, palette, density, barHeight)
    private val resizeHandle = ResizeHandle(
        palette, density, barHeight,
        onDrag = listener::onResizeDrag,
        onDone = { resizing = false },
    )

    private var resizing = false
        set(value) {
            field = value
            invalidate()
        }

    private val popup = ChoicePopup(this, palette)

    private val divider = Paint().apply {
        color = palette.faint
        strokeWidth = maxOf(1f, density * 0.7f)
    }

    /** 候选右端 ⌄ 那一格的宽；候选滚到它底下为止。 */
    private val expandSlot = 46f * density
    private val expandFill = Paint()
    private val expandIcon = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = palette.text }

    private val expandable: Boolean get() = strip.candidates.isNotEmpty()

    /** 显示的是工具栏：横滑滑的是它中间那排图标，不是整条。 */
    private val toolbarShown: Boolean get() = !resizing && strip.isEmpty

    /** 候选能用的宽度：有 ⌄ 时让出那一格。 */
    private val stripWidth: Int get() = if (expandable) width - expandSlot.toInt() else width

    /** 气泡对着的候选下标；气泡开着时高亮跟到它上面。 */
    private var popupIndex = -1

    /** 气泡里「删除」那一项的下标；这个候选删不了时为 -1。 */
    private var forgetItem = -1

    /** 气泡里只有「删除」（候选没有译文）。 */
    private var forgetOnly = false

    private val scroller = OverScroller(context)

    private val gestures = GestureDetector(context, object : GestureDetector.SimpleOnGestureListener() {
        override fun onDown(e: MotionEvent): Boolean {
            scroller.forceFinished(true)
            return true
        }

        override fun onScroll(e1: MotionEvent?, e2: MotionEvent, dx: Float, dy: Float): Boolean {
            if (toolbarShown) {
                toolbar.scrollTo(toolbar.offset + dx, width)
                invalidate()
                return true
            }
            scrollTo((scrollX + dx).toInt().coerceIn(0, maxScroll()), 0)
            return true
        }

        override fun onFling(e1: MotionEvent?, e2: MotionEvent, vx: Float, vy: Float): Boolean {
            if (toolbarShown) {
                scroller.fling(toolbar.offset.toInt(), 0, -vx.toInt(), 0, 0, toolbar.maxOffset(width).toInt(), 0, 0)
            } else {
                scroller.fling(scrollX, 0, -vx.toInt(), 0, 0, maxScroll(), 0, 0)
            }
            postInvalidateOnAnimation()
            return true
        }

        override fun onLongPress(e: MotionEvent) = openTranslations(e)

        override fun onSingleTapUp(e: MotionEvent): Boolean {
            tap(e.x)
            return true
        }
    })

    init {
        setBackgroundColor(palette.board)
    }

    /** 拼音行右侧的提示（删候选的结果）由渲染器从 Rust 那边的帧里取，这里只要拼音和候选。 */
    fun show(preedit: String, candidates: List<Candidate>) {
        closePopup()
        strip.show(preedit, candidates)
        scroller.forceFinished(true)
        scrollTo(0, 0)
        repaint()
    }

    /** 进入拖动调高度（功能面板里点「键盘高度」）。 */
    fun startResize() {
        resizing = true
    }

    /** 按当前高亮重画位图：新的一帧到了、或长按气泡换了对着的候选。 */
    private fun repaint() {
        strip.paint(if (strip.isEmpty) null else source.render(popupIndex.coerceAtLeast(0), palette.dark, stripWidth))
        invalidate()
    }

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        setMeasuredDimension(MeasureSpec.getSize(widthMeasureSpec), barHeight.toInt())
    }

    override fun computeScroll() {
        if (scroller.computeScrollOffset()) {
            if (toolbarShown) toolbar.scrollTo(scroller.currX.toFloat(), width) else scrollTo(scroller.currX, 0)
            postInvalidateOnAnimation()
        }
    }

    override fun onDraw(canvas: Canvas) = when {
        resizing -> resizeHandle.draw(canvas, width)
        strip.isEmpty -> toolbar.draw(canvas, width)
        else -> {
            strip.draw(canvas, scrollX, stripWidth)
            if (expandable) drawExpand(canvas)
            // 候选条与键盘之间一道细线，分出层次；滚动时线不跟着走
            canvas.drawRect(scrollX.toFloat(), height - divider.strokeWidth, scrollX + width.toFloat(), height.toFloat(), divider)
        }
    }

    @SuppressLint("ClickableViewAccessibility")
    override fun onTouchEvent(event: MotionEvent): Boolean = when {
        resizing -> resizeHandle.onTouch(event, width)
        popup.showing -> onPopupTouch(event)
        else -> gestures.onTouchEvent(event)
    }

    /** 钉在右端的 ⌄：与候选行同一高度居中，左边一道短竖线隔开。 */
    private fun drawExpand(canvas: Canvas) {
        val left = scrollX + stripWidth.toFloat()
        expandFill.color = strip.background
        canvas.drawRect(left, 0f, scrollX + width.toFloat(), height.toFloat(), expandFill)
        val cy = (strip.candidatesTop + height) / 2f
        canvas.drawRect(left, cy - 12 * density, left + divider.strokeWidth, cy + 12 * density, divider)
        PanelIcons.draw(canvas, PanelIcons.Icon.EXPAND, left + expandSlot / 2, cy, 24 * density, expandIcon)
    }

    private fun tap(x: Float) {
        if (expandable && x >= stripWidth) {
            listener.onExpand()
            return
        }
        if (!strip.isEmpty) {
            strip.indexAt(x + scrollX).takeIf { it >= 0 }?.let(listener::onSelect)
            return
        }
        when (val action = toolbar.actionAt(x, width)) {
            Toolbar.Action.RESIZE -> resizing = true
            else -> listener.onToolbar(action)
        }
    }

    /**
     * 长按候选：弹出它的译词，第一条对着手指；删得掉的候选（用户词、学过的词）末尾再跟一项「删除」，
     * 对应桌面端修饰键 + 数字。没有译文也删不了的候选不弹。
     */
    private fun openTranslations(event: MotionEvent) {
        if (event.x >= stripWidth) return
        val index = strip.indexAt(event.x + scrollX)
        val candidate = strip.candidates.getOrNull(index) ?: return
        val senses = candidate.senses
        if (senses.isEmpty() && !candidate.forgettable) return
        popupIndex = index
        repaint()
        Haptics.hold(context)
        val items = senses.map { sense ->
            ChoicePopup.Item(sense.text, listOfNotNull(sense.pos, sense.reading).joinToString(" "), sense.fresh)
        } + if (candidate.forgettable) listOf(ChoicePopup.Item(context.getString(R.string.candidate_forget), context.getString(R.string.candidate_forget_note))) else emptyList()
        forgetItem = if (candidate.forgettable) senses.size else -1
        forgetOnly = senses.isEmpty()
        // 只有「删除」时不预选，得滑上去才算，免得长按一松手就删了
        popup.show(items, initial = if (senses.isEmpty()) -1 else 0, fingerScreenX = event.rawX, bottom = 0f)
    }

    /** 气泡开着：横向移动只用来选项，抬手上屏；往下滑进键盘算取消。 */
    private fun onPopupTouch(event: MotionEvent): Boolean {
        when (event.actionMasked) {
            MotionEvent.ACTION_MOVE ->
                // 只有「删除」时手指得往上滑进气泡才算选中
                popup.track(event.rawX, cancelled = event.y > height + 24 * density || (forgetOnly && event.y > 0))

            MotionEvent.ACTION_UP -> {
                val index = popupIndex
                val forget = forgetItem
                closePopup()?.let { item ->
                    if (item == forget) listener.onForget(index) else listener.onSelectTranslation(index, item)
                }
            }

            MotionEvent.ACTION_CANCEL -> closePopup()
        }
        return true
    }

    private fun closePopup(): Int? {
        val sense = if (popup.showing) popup.finish() else null
        // 气泡对着的不是第一个候选时，高亮要回到第一个
        val moved = popupIndex > 0
        popupIndex = -1
        if (moved) repaint()
        return sense
    }

    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()
        closePopup()
    }

    private fun maxScroll(): Int = (strip.contentWidth - stripWidth).toInt().coerceAtLeast(0)
}
