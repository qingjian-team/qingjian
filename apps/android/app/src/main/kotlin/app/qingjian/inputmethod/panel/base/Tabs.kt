package app.qingjian.inputmethod.panel.base

import android.graphics.Canvas
import android.graphics.RectF
import android.view.MotionEvent

/**
 * 一排页签。[Style.UNDERLINE] 是面板顶上的大类（选中加粗、下面一道品牌色短线，放得下就均分）；
 * [Style.PILL] 是底下的小类（选中垫一块圆角底，从左排起，排不下可以横着拖）；
 * [Style.COLUMN] 同 [Style.PILL] 但竖着排成一列（左手布局符号面板的分类），排不下竖着拖。
 */
class Tabs(private val ink: Ink, private val scroll: ScrollState, private val style: Style) {
    enum class Style { UNDERLINE, PILL, COLUMN }

    var labels: List<String> = emptyList()
        set(value) {
            field = value
            scroll.reset()
            arranged = false
        }

    var selected = 0

    val bounds = RectF()

    /** 每个页签沿排列方向的起点与长度（横排是左与宽，竖排是上与高）。 */
    private var starts = FloatArray(0)
    private var sizes = FloatArray(0)
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
        starts = FloatArray(labels.size)
        if (style == Style.COLUMN) {
            val each = ink.dp(COLUMN_ROW_DP)
            sizes = FloatArray(labels.size) { each }
            for (i in labels.indices) starts[i] = bounds.top + each * i
            scroll.max = each * labels.size - bounds.height()
            return
        }
        val padding = ink.dp(if (style == Style.UNDERLINE) 18f else 14f)
        sizes = FloatArray(labels.size) { ink.measure(labels[it], textSize, bold = true) + padding * 2 }
        val total = sizes.sum()
        if (style == Style.UNDERLINE && total <= bounds.width()) {
            val each = bounds.width() / labels.size.coerceAtLeast(1)
            for (i in labels.indices) {
                starts[i] = bounds.left + each * i
                sizes[i] = each
            }
            scroll.max = 0f
        } else {
            var x = bounds.left + ink.dp(6f)
            for (i in labels.indices) {
                starts[i] = x
                x += sizes[i]
            }
            scroll.max = x + ink.dp(6f) - bounds.right
        }
    }

    fun draw(canvas: Canvas) {
        arrange()
        canvas.save()
        canvas.clipRect(bounds)
        if (style == Style.COLUMN) drawColumn(canvas) else drawRow(canvas)
        canvas.restore()
    }

    private fun drawRow(canvas: Canvas) {
        val palette = ink.palette
        val cy = bounds.centerY()
        for (i in labels.indices) {
            val left = starts[i] - scroll.offset
            if (left > bounds.right || left + sizes[i] < bounds.left) continue
            val cx = left + sizes[i] / 2
            val chosen = i == selected
            if (style == Style.PILL && chosen) {
                pill.set(left + ink.dp(3f), cy - ink.dp(16f), left + sizes[i] - ink.dp(3f), cy + ink.dp(16f))
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
    }

    private fun drawColumn(canvas: Canvas) {
        val palette = ink.palette
        val cx = bounds.centerX()
        for (i in labels.indices) {
            val top = starts[i] - scroll.offset
            if (top > bounds.bottom || top + sizes[i] < bounds.top) continue
            val chosen = i == selected
            if (chosen) {
                pill.set(bounds.left + ink.dp(4f), top + ink.dp(4f), bounds.right - ink.dp(4f), top + sizes[i] - ink.dp(4f))
                ink.round(canvas, pill, 9f, palette.accent)
            }
            ink.text(
                canvas, labels[i], 14f, if (chosen) palette.text else palette.gloss, cx, top + sizes[i] / 2,
                bold = chosen, maxWidth = bounds.width() - ink.dp(8f),
            )
        }
    }

    fun contains(event: MotionEvent) = bounds.contains(event.x, event.y)

    /** 横排按 [x]、竖排按 [y] 找落在哪个页签上。 */
    fun indexAt(x: Float, y: Float = bounds.centerY()): Int? {
        arrange()
        val at = (if (style == Style.COLUMN) y else x) + scroll.offset
        return labels.indices.firstOrNull { at >= starts[it] && at < starts[it] + sizes[it] }
    }

    private companion object {
        /** 竖排一个页签的高度。 */
        const val COLUMN_ROW_DP = 46f
    }
}
