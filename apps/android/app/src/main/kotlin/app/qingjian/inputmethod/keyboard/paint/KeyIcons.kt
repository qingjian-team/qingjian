package app.qingjian.inputmethod.keyboard.paint

import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path
import app.qingjian.inputmethod.keyboard.layout.Key

/** 功能键的矢量图标，画在 24×24 的格子里再按 [size] 缩放；不用字体里的符号，各家系统字体画出来不一样。 */
object KeyIcons {
    private val path = Path()

    fun draw(canvas: Canvas, key: Key, cx: Float, cy: Float, size: Float, paint: Paint, filled: Boolean) {
        val build = when (key) {
            Key.Shift -> ::shift
            Key.Backspace -> ::backspace
            is Key.Enter -> ::enter
            else -> return
        }
        path.rewind()
        build(path)
        val scale = size / 24f
        canvas.save()
        canvas.translate(cx - size / 2, cy - size / 2)
        canvas.scale(scale, scale)
        paint.strokeWidth = 1.8f
        paint.strokeJoin = Paint.Join.ROUND
        paint.strokeCap = Paint.Cap.ROUND
        paint.style = if (filled && key == Key.Shift) Paint.Style.FILL_AND_STROKE else Paint.Style.STROKE
        canvas.drawPath(path, paint)
        paint.style = Paint.Style.FILL
        canvas.restore()
    }

    private fun shift(p: Path) {
        p.moveTo(12f, 4f)
        p.lineTo(20f, 12.5f)
        p.lineTo(15.5f, 12.5f)
        p.lineTo(15.5f, 19.5f)
        p.lineTo(8.5f, 19.5f)
        p.lineTo(8.5f, 12.5f)
        p.lineTo(4f, 12.5f)
        p.close()
    }

    private fun backspace(p: Path) {
        p.moveTo(9f, 5.5f)
        p.lineTo(21f, 5.5f)
        p.lineTo(21f, 18.5f)
        p.lineTo(9f, 18.5f)
        p.lineTo(3f, 12f)
        p.close()
        p.moveTo(12f, 9.5f)
        p.lineTo(17f, 14.5f)
        p.moveTo(17f, 9.5f)
        p.lineTo(12f, 14.5f)
    }

    private fun enter(p: Path) {
        p.moveTo(19f, 6f)
        p.lineTo(19f, 14f)
        p.lineTo(6f, 14f)
        p.moveTo(10f, 9.5f)
        p.lineTo(5.5f, 14f)
        p.lineTo(10f, 18.5f)
    }
}
