package app.qingjian.inputmethod.settings.widget

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Rect
import android.graphics.RectF
import android.view.View
import app.qingjian.inputmethod.theme.Palette

/**
 * 主题的样子：上面贴渲染器画的候选条（超出的截掉），下面按键盘配色画几排没有字的键，回车键是品牌色。
 * 外观页的大预览与主题卡片共用；高按宽的比例 [aspect] 定。
 */
@SuppressLint("ViewConstructor")
class ThemePreview(
    context: Context,
    private val strip: Bitmap?,
    private val palette: Palette,
    /** 键盘那一截的高与宽之比。 */
    private val aspect: Float,
    /** 键的圆角（dp）。 */
    private val keyRadiusDp: Float,
) : View(context) {
    private val density = resources.displayMetrics.density
    private val fill = Paint(Paint.ANTI_ALIAS_FLAG)
    private val src = Rect()
    private val dst = Rect()
    private val key = RectF()

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val width = MeasureSpec.getSize(widthMeasureSpec)
        val stripHeight = strip?.height ?: 0
        setMeasuredDimension(width, stripHeight + (width * aspect).toInt())
    }

    override fun onDraw(canvas: Canvas) {
        canvas.drawColor(palette.board)
        val stripHeight = strip?.height ?: 0
        strip?.let { bitmap ->
            fill.color = palette.surface
            canvas.drawRect(0f, 0f, width.toFloat(), stripHeight.toFloat(), fill)
            src.set(0, 0, minOf(width, bitmap.width), bitmap.height)
            dst.set(src)
            canvas.drawBitmap(bitmap, src, dst, null)
        }
        drawKeys(canvas, stripHeight.toFloat(), height.toFloat())
    }

    /** 四排：10 个字母、9 个字母、功能键夹 7 个字母、底排 功能 / 空格 / 回车。 */
    private fun drawKeys(canvas: Canvas, top: Float, bottom: Float) {
        val gap = width / 80f
        val rowHeight = (bottom - top - gap) / 4
        val unit = (width - gap) / 10f
        val radius = keyRadiusDp * density
        fun draw(left: Float, row: Int, units: Float, color: Int) {
            val y = top + gap + row * rowHeight
            key.set(left + gap, y, left + units * unit, y + rowHeight - gap)
            fill.color = palette.keyShadow
            canvas.drawRoundRect(key.left, key.top + gap / 3, key.right, key.bottom + gap / 3, radius, radius, fill)
            fill.color = color
            canvas.drawRoundRect(key, radius, radius, fill)
        }
        for (i in 0 until 10) draw(i * unit, 0, 1f, palette.surface)
        for (i in 0 until 9) draw((i + 0.5f) * unit, 1, 1f, palette.surface)
        draw(0f, 2, 1.5f, palette.functionalKey)
        for (i in 0 until 7) draw((i + 1.5f) * unit, 2, 1f, palette.surface)
        draw(8.5f * unit, 2, 1.5f, palette.functionalKey)
        draw(0f, 3, 2.5f, palette.functionalKey)
        draw(2.5f * unit, 3, 5f, palette.surface)
        draw(7.5f * unit, 3, 2.5f, palette.brand)
    }
}
