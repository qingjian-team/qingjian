package app.qingjian.inputmethod.panel.base

import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import android.text.TextPaint
import android.text.TextUtils
import app.qingjian.inputmethod.theme.Palette

/** 面板共用的画笔：居中的字、图标、圆角块。尺寸参数都是 dp / sp，这里乘密度。 */
class Ink(val palette: Palette, val density: Float) {
    val fill = Paint(Paint.ANTI_ALIAS_FLAG)
    private val text = TextPaint(Paint.ANTI_ALIAS_FLAG).apply { textAlign = Paint.Align.CENTER }
    private val stroke = Paint(Paint.ANTI_ALIAS_FLAG)

    fun dp(value: Float) = value * density

    fun measure(value: String, sp: Float, bold: Boolean = false): Float {
        text.textSize = sp * density
        text.isFakeBoldText = bold
        return text.measureText(value)
    }

    /** 以 ([cx], [cy]) 为中心画一行字；给了 [maxWidth] 就超宽截断加省略号。 */
    fun text(
        canvas: Canvas, value: String, sp: Float, color: Int, cx: Float, cy: Float,
        bold: Boolean = false, maxWidth: Float = 0f,
    ) {
        text.textSize = sp * density
        text.color = color
        text.isFakeBoldText = bold
        val shown = if (maxWidth > 0f) {
            TextUtils.ellipsize(value, text, maxWidth, TextUtils.TruncateAt.END).toString()
        } else {
            value
        }
        canvas.drawText(shown, cx, cy - (text.descent() + text.ascent()) / 2, text)
    }

    /** 从 [x] 起往右画一行字（纵向以 [cy] 居中），超出 [right] 截断加省略号。 */
    fun textStart(canvas: Canvas, value: String, sp: Float, color: Int, x: Float, cy: Float, right: Float) {
        text.textSize = sp * density
        text.color = color
        text.isFakeBoldText = false
        text.textAlign = Paint.Align.LEFT
        val shown = TextUtils.ellipsize(value, text, (right - x).coerceAtLeast(0f), TextUtils.TruncateAt.END).toString()
        canvas.drawText(shown, x, cy - (text.descent() + text.ascent()) / 2, text)
        text.textAlign = Paint.Align.CENTER
    }

    fun icon(canvas: Canvas, icon: PanelIcons.Icon, cx: Float, cy: Float, sizeDp: Float, color: Int) {
        stroke.color = color
        PanelIcons.draw(canvas, icon, cx, cy, dp(sizeDp), stroke)
    }

    /**
     * 主题是不是方角（墨岩这种把面板圆角写成 0 的主题）。
     *
     * 方角主题下所有面板块都不许留圆角：调用点还按各自的缺省写着 8f / 12f / 16f，
     * 由 [round] 统一抹平，省得每一处都记着判断一次。
     */
    val square: Boolean get() = palette.boardRadius == 0f

    /** 实际画出来的圆角：方角主题一律 0，其余用调用点给的值。 */
    fun corner(radiusDp: Float): Float = if (square) 0f else radiusDp

    fun round(canvas: Canvas, rect: RectF, radiusDp: Float, color: Int) {
        fill.color = color
        val radius = dp(corner(radiusDp))
        canvas.drawRoundRect(rect, radius, radius, fill)
    }

    fun line(canvas: Canvas, x0: Float, y0: Float, x1: Float, y1: Float, color: Int) {
        fill.color = color
        fill.strokeWidth = maxOf(1f, density * 0.6f)
        canvas.drawLine(x0, y0, x1, y1, fill)
    }
}
