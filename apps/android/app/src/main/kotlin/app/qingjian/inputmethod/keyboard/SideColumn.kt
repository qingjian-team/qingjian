package app.qingjian.inputmethod.keyboard

import android.graphics.RectF
import android.view.MotionEvent
import kotlin.math.abs

/**
 * 九宫格左边那一列：组句中是同码的拼音，平时是标点。项目多了可以上下拖，没拖就算点。
 * 只认自己那根手指，别的手指照常按键。
 */
class SideColumn(private val density: Float, private val onItem: (String) -> Unit) {
    var enabled = false
    var bounds = RectF()

    var items: List<String> = emptyList()
        set(value) {
            if (value == field) return
            field = value
            scroll = 0f
        }

    private var scroll = 0f
    private var pointer = -1
    private var lastY = 0f
    private var dragged = false

    private val itemHeight get() = bounds.height() / VISIBLE_ITEMS
    private val maxScroll get() = (items.size * itemHeight - bounds.height()).coerceAtLeast(0f)

    /** 逐个给出看得见的项目和它的中线 y。 */
    fun visibleItems(draw: (String, Float) -> Unit) {
        items.forEachIndexed { index, item ->
            val cy = bounds.top + itemHeight * (index + 0.5f) - scroll
            if (cy > bounds.top - itemHeight && cy < bounds.bottom + itemHeight) draw(item, cy)
        }
    }

    /** 返回 true 表示这个事件归拼音列，键盘不用再管；内容变了（滚动）由 [invalidate] 通知。 */
    fun onTouch(event: MotionEvent, invalidate: () -> Unit): Boolean {
        val index = event.actionIndex
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN, MotionEvent.ACTION_POINTER_DOWN -> {
                if (!enabled || !bounds.contains(event.getX(index), event.getY(index))) return false
                pointer = event.getPointerId(index)
                lastY = event.getY(index)
                dragged = false
                return true
            }

            MotionEvent.ACTION_MOVE -> {
                val at = event.findPointerIndex(pointer)
                if (at < 0) return false
                val dy = lastY - event.getY(at)
                if (dragged || abs(dy) > DRAG_SLOP_DP * density) {
                    dragged = true
                    scroll = (scroll + dy).coerceIn(0f, maxScroll)
                    lastY = event.getY(at)
                    invalidate()
                }
                // 别的手指可能正按着键，移动事件接着往下传
                return false
            }

            MotionEvent.ACTION_UP, MotionEvent.ACTION_POINTER_UP -> {
                if (event.getPointerId(index) != pointer) return false
                pointer = -1
                if (!dragged) {
                    val item = ((event.getY(index) - bounds.top + scroll) / itemHeight).toInt()
                    items.getOrNull(item)?.let(onItem)
                }
                return true
            }

            MotionEvent.ACTION_CANCEL -> pointer = -1
        }
        return false
    }

    private companion object {
        const val VISIBLE_ITEMS = 4.5f
        const val DRAG_SLOP_DP = 8
    }
}
