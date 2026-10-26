package app.qingjian.inputmethod.panel.base

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.RectF
import android.view.GestureDetector
import android.view.MotionEvent
import android.view.View
import app.qingjian.inputmethod.keyboard.Haptics
import app.qingjian.inputmethod.panel.PanelActions
import app.qingjian.inputmethod.theme.Palette

/**
 * 面板的底子：盖住候选条与键盘那一整块。顶上一条与工具栏同高的表头（左边返回、中间页签、右边可选一个图标），
 * 下面由子类画。拖动交给手指落下处的那块 [ScrollState]（横或竖），没拖动就算点。
 */
@SuppressLint("ViewConstructor")
abstract class PanelView(
    context: Context,
    palette: Palette,
    protected val actions: PanelActions,
    /** 表头高度，与候选条一样高，盖上去时返回键正好在 logo 的位置。 */
    protected val headerHeight: Float,
) : View(context) {
    protected val ink = Ink(palette, resources.displayMetrics.density)
    protected val palette get() = ink.palette

    /** 一块可滚动区域：落在 [contains] 里的拖动归它。 */
    protected class Scrollable(val state: ScrollState, val horizontal: Boolean, val contains: (Float, Float) -> Boolean)

    private val scrollables = mutableListOf<Scrollable>()
    private val headerCard = RectF()
    private var dragging: Scrollable? = null

    protected fun scrollable(horizontal: Boolean, contains: (Float, Float) -> Boolean): ScrollState {
        val state = ScrollState(context)
        scrollables += Scrollable(state, horizontal, contains)
        return state
    }

    private val gestures = GestureDetector(context, object : GestureDetector.SimpleOnGestureListener() {
        override fun onDown(e: MotionEvent): Boolean {
            scrollables.forEach { it.state.stop() }
            dragging = scrollables.firstOrNull { it.contains(e.x, e.y) }
            return true
        }

        override fun onScroll(e1: MotionEvent?, e2: MotionEvent, dx: Float, dy: Float): Boolean {
            val target = dragging ?: return false
            target.state.drag(if (target.horizontal) dx else dy)
            invalidate()
            return true
        }

        override fun onFling(e1: MotionEvent?, e2: MotionEvent, vx: Float, vy: Float): Boolean {
            val target = dragging ?: return false
            target.state.fling(if (target.horizontal) vx else vy)
            postInvalidateOnAnimation()
            return true
        }

        override fun onLongPress(e: MotionEvent) {
            dragging = null
            onHold(e.x, e.y, e.rawX)
        }

        override fun onSingleTapUp(e: MotionEvent): Boolean {
            Haptics.tap(context)
            if (hasHeader && e.y < headerHeight && e.x < ink.dp(BACK_WIDTH_DP)) onBack() else onTap(e.x, e.y)
            invalidate()
            return true
        }
    })

    /** 有没有表头；没有的（符号面板）左上角不当返回键。 */
    protected open val hasHeader = true

    /** 长按（没拖动）；之后这根手指的移动与抬起由子类在 [onTouchEvent] 里接着收。 */
    protected open fun onHold(x: Float, y: Float, rawX: Float) = Unit

    /** 点了表头的返回：缺省回键盘，从别的面板进来的可以回那个面板。 */
    protected open fun onBack() = actions.closePanel()

    /** 点在返回键以外的地方。 */
    protected abstract fun onTap(x: Float, y: Float)

    init {
        setBackgroundColor(palette.board)
    }

    override fun computeScroll() {
        var moving = false
        for (scrollable in scrollables) moving = scrollable.state.compute() || moving
        if (moving) postInvalidateOnAnimation()
    }

    @SuppressLint("ClickableViewAccessibility")
    override fun onTouchEvent(event: MotionEvent): Boolean = gestures.onTouchEvent(event)

    /**
     * 表头：与工具栏同样浮起的圆角卡片，和下面的内容分出层次；左边返回，[rightIcon] 画在右边一格
     * （点它由子类在 [onTap] 里认 [isRightIcon]）。
     */
    protected fun drawHeader(canvas: Canvas, tabs: Tabs?, rightIcon: PanelIcons.Icon? = null) {
        val inset = ink.dp(6f)
        headerCard.set(inset, inset, width - inset, headerHeight - ink.dp(4f))
        headerCard.offset(0f, ink.density)
        ink.round(canvas, headerCard, 14f, palette.keyShadow)
        headerCard.offset(0f, -ink.density)
        ink.round(canvas, headerCard, 14f, palette.surface)
        val cy = headerCard.centerY()
        ink.icon(canvas, PanelIcons.Icon.BACK, ink.dp(BACK_WIDTH_DP) / 2, cy, 24f, palette.text)
        tabs?.let {
            it.setBounds(ink.dp(BACK_WIDTH_DP), headerCard.top, width - ink.dp(BACK_WIDTH_DP), headerCard.bottom)
            it.draw(canvas)
        }
        rightIcon?.let { ink.icon(canvas, it, width - ink.dp(BACK_WIDTH_DP) / 2, cy, 24f, palette.gloss) }
    }

    protected fun isRightIcon(x: Float, y: Float) = y < headerHeight && x > width - ink.dp(BACK_WIDTH_DP)

    protected companion object {
        const val BACK_WIDTH_DP = 56f
    }
}
