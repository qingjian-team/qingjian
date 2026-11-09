package app.qingjian.inputmethod.panel.base

import android.content.Context
import android.widget.OverScroller

/** 面板里一块可滚动区域的滚动量：拖动直接加，松手按速度惯性滚，[compute] 每帧推进。 */
class ScrollState(context: Context) {
    private val scroller = OverScroller(context)

    var offset = 0f
        private set

    var max = 0f
        set(value) {
            field = value.coerceAtLeast(0f)
            offset = offset.coerceIn(0f, field)
        }

    fun reset() {
        scroller.forceFinished(true)
        offset = 0f
    }

    fun stop() = scroller.forceFinished(true)

    fun drag(delta: Float) {
        offset = (offset + delta).coerceIn(0f, max)
    }

    fun fling(velocity: Float) {
        scroller.fling(0, offset.toInt(), 0, -velocity.toInt(), 0, 0, 0, max.toInt())
    }

    /** 惯性还在滚返回 true，调用方接着要下一帧。 */
    fun compute(): Boolean {
        if (!scroller.computeScrollOffset()) return false
        offset = scroller.currY.toFloat().coerceIn(0f, max)
        return true
    }
}
