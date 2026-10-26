package app.qingjian.inputmethod.settings.widget

import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import android.view.View
import app.qingjian.inputmethod.theme.AppColors

/**
 * 统计页的柱状图：一列一个数，最后一根（现在）用品牌色，其余半透明；底下在 [labels] 给的位置写刻度。
 * 全是 0 时只画底线。高度由外面定。
 */
class BarChart(
    context: Context,
    private val colors: AppColors,
    private val values: List<Long>,
    /** 列下标 → 刻度文字。 */
    private val labels: Map<Int, String>,
) : View(context) {
    private val density = resources.displayMetrics.density
    private val bar = Paint(Paint.ANTI_ALIAS_FLAG)
    private val text = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = colors.secondary
        textSize = 10.5f * resources.displayMetrics.scaledDensity
        textAlign = Paint.Align.CENTER
    }
    private val line = Paint().apply { color = colors.divider }
    private val rect = RectF()

    override fun onDraw(canvas: Canvas) {
        if (values.isEmpty()) return
        val labelHeight = text.textSize + 8 * density
        val bottom = height - labelHeight
        val slot = width.toFloat() / values.size
        val gap = (slot * 0.28f).coerceAtMost(6 * density)
        val max = values.max().coerceAtLeast(1)
        canvas.drawRect(0f, bottom, width.toFloat(), bottom + density, line)
        values.forEachIndexed { index, value ->
            if (value > 0) {
                val top = bottom - (bottom - 4 * density) * value / max
                rect.set(index * slot + gap / 2, top.coerceAtMost(bottom - 2 * density), (index + 1) * slot - gap / 2, bottom)
                bar.color = if (index == values.lastIndex) colors.brand else colors.brand and 0x00FFFFFF or 0x66000000
                canvas.drawRoundRect(rect, 2 * density, 2 * density, bar)
            }
        }
        labels.forEach { (index, label) ->
            val x = (index * slot + slot / 2).coerceIn(text.measureText(label) / 2, width - text.measureText(label) / 2)
            canvas.drawText(label, x, height - 4 * density, text)
        }
    }

    companion object {
        /** 按天的柱子标第一天、中间一天与今天（月/日）；[dates] 是 `2026-10-19` 这样的日期，早的在前。 */
        fun dayLabels(dates: List<String>): Map<Int, String> {
            if (dates.isEmpty()) return emptyMap()
            fun label(index: Int) = dates[index].split('-').let { "${it[1].toInt()}/${it[2].toInt()}" }
            return listOf(0, dates.size / 2, dates.lastIndex).associateWith(::label)
        }
    }
}
