package app.qingjian.inputmethod.panel.base

import android.graphics.Canvas
import android.graphics.RectF
import android.view.MotionEvent

/**
 * 一排页签。[Style.UNDERLINE] 是面板顶上的大类（选中加粗、下面一道品牌色短线，放得下就均分）；
 * [Style.PILL] 是底下的小类（选中垫一块圆角底，从左排起，排不下可以横着拖）。
 */
class Tabs(private val ink: Ink, private val scroll: ScrollState, private val style: Style) {
    enum class Style { UNDERLINE, PILL }

    var labels: List<String> = emptyList()
        set(value) {
            field = value
            scroll.reset()
            arranged = false
        }

    var selected = 0

    val bounds = RectF()

    private var lefts = FloatArray(0)
    private var widths = FloatArray(0)
    private var arranged = false
    private val pill = RectF()

    private val textSize get() = if (style == Style.UNDERLINE) 16f else 15f

    fun setBounds(left: Float, top: Float, right: Float, bottom: Float) {
        bounds.set(left, top, right, bottom)
        arranged = false
    }

    private fun arrange() {
        if (arranged) return
        arranged = true
        val padding = ink.dp(if (style == Style.UNDERLINE) 18f else 14f)
        widths = FloatArray(labels.size) { ink.measure(labels[it], textSize, bold = true) + padding * 2 }
        val total = widths.sum()
        lefts = FloatArray(labels.size)
        if (style == Style.UNDERLINE && total <= bounds.width()) {
            val each = bounds.width() / labels.size.coerceAtLeast(1)
            for (i in labels.indices) {
                lefts[i] = bounds.left + each * i
                widths[i] = each
            }
            scroll.max = 0f
        } else {
            var x = bounds.left + ink.dp(6f)
            for (i in labels.indices) {
                lefts[i] = x
                x += widths[i]
            }
            scroll.max = x + ink.dp(6f) - bounds.right
        }
    }

    fun draw(canvas: Canvas) {
        arrange()
        canvas.save()
        canvas.clipRect(bounds)
        val palette = ink.palette
        val cy = bounds.centerY()
        for (i in labels.indices) {
            val left = lefts[i] - scroll.offset
            if (left > bounds.right || left + widths[i] < bounds.left) continue
            val cx = left + widths[i] / 2
            val chosen = i == selected
            if (style == Style.PILL && chosen) {
                pill.set(left + ink.dp(3f), cy - ink.dp(16f), left + widths[i] - ink.dp(3f), cy + ink.dp(16f))
                ink.round(canvas, pill, 9f, palette.accent)
            }
            ink.text(
                canvas, labels[i], textSize,
                if (chosen) palette.text else palette.gloss, cx, cy, bold = chosen,
            )
            if (style == Style.UNDERLINE && chosen) {
                pill.set(cx - ink.dp(13f), bounds.bottom - ink.dp(9f), cx + ink.dp(13f), bounds.bottom - ink.dp(6f))
                ink.round(canvas, pill, 1.5f, palette.brand)
            }
        }
        canvas.restore()
    }

    fun contains(event: MotionEvent) = bounds.contains(event.x, event.y)

    fun indexAt(x: Float): Int? {
        arrange()
        val at = x + scroll.offset
        return labels.indices.firstOrNull { at >= lefts[it] && at < lefts[it] + widths[it] }
    }
}
