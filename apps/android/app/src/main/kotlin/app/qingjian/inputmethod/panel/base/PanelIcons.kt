package app.qingjian.inputmethod.panel.base

import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path
import android.graphics.RectF

/** 工具栏、面板与 App 页面的线条图标，画在 24×24 的格子里再按尺寸缩放；与 [app.qingjian.inputmethod.keyboard.paint.KeyIcons] 同一种画法。 */
object PanelIcons {
    enum class Icon { BACK, CHEVRON, EXPAND, COLLAPSE, HIDE, EMOJI, KEYBOARD, SYMBOLS, RESIZE, MIC, CLIPBOARD, TRANSLATE, CLEAR, PLUS, TRASH, PEN, TOP, SPLIT, COPY, SETTINGS, GLOBE, LOCK, UNLOCK, CHECK, INFO, HOME, BOOK, USER, CHART, CLOUD, PALETTE }

    private val path = Path()
    private val oval = RectF()

    fun draw(canvas: Canvas, icon: Icon, cx: Float, cy: Float, size: Float, paint: Paint) {
        path.rewind()
        when (icon) {
            Icon.BACK -> polyline(15f, 5f, 8f, 12f, 15f, 19f)
            Icon.CHEVRON -> polyline(9.5f, 6f, 15.5f, 12f, 9.5f, 18f)
            Icon.EXPAND -> polyline(6f, 9.5f, 12f, 15.5f, 18f, 9.5f)
            Icon.COLLAPSE -> polyline(6f, 14.5f, 12f, 8.5f, 18f, 14.5f)
            Icon.HIDE -> polyline(5f, 9f, 12f, 16f, 19f, 9f)
            Icon.EMOJI -> emoji()
            Icon.KEYBOARD -> keyboard()
            Icon.SYMBOLS -> symbols()
            Icon.RESIZE -> resize()
            Icon.MIC -> microphone()
            Icon.CLIPBOARD -> clipboard()
            Icon.TRANSLATE -> translate()
            Icon.CLEAR -> { circle(12f, 12f, 9f); polyline(9f, 9f, 15f, 15f); polyline(15f, 9f, 9f, 15f) }
            Icon.PLUS -> { polyline(12f, 5f, 12f, 19f); polyline(5f, 12f, 19f, 12f) }
            Icon.TRASH -> trash()
            Icon.PEN -> polyline(5f, 19f, 5.5f, 15.5f, 15.5f, 5.5f, 18.5f, 8.5f, 8.5f, 18.5f, 5f, 19f)
            Icon.TOP -> { polyline(6f, 5f, 18f, 5f); polyline(12f, 20f, 12f, 9f); polyline(7.5f, 13f, 12f, 8.5f, 16.5f, 13f) }
            Icon.SPLIT -> split()
            Icon.COPY -> copy()
            Icon.SETTINGS -> settings()
            Icon.GLOBE -> globe()
            Icon.LOCK -> lock(open = false)
            Icon.UNLOCK -> lock(open = true)
            Icon.CHECK -> polyline(6f, 12.5f, 10.5f, 17f, 18.5f, 8f)
            Icon.INFO -> info()
            Icon.HOME -> home()
            Icon.BOOK -> book()
            Icon.USER -> user()
            Icon.CHART -> chart()
            Icon.CLOUD -> cloud()
            Icon.PALETTE -> palette()
        }
        val scale = size / 24f
        canvas.save()
        canvas.translate(cx - size / 2, cy - size / 2)
        canvas.scale(scale, scale)
        val style = paint.style
        val width = paint.strokeWidth
        paint.style = Paint.Style.STROKE
        paint.strokeWidth = 1.7f
        paint.strokeJoin = Paint.Join.ROUND
        paint.strokeCap = Paint.Cap.ROUND
        canvas.drawPath(path, paint)
        paint.style = style
        paint.strokeWidth = width
        canvas.restore()
    }

    private fun polyline(vararg points: Float) {
        path.moveTo(points[0], points[1])
        for (i in 2 until points.size step 2) path.lineTo(points[i], points[i + 1])
    }

    private fun circle(cx: Float, cy: Float, r: Float) = path.addCircle(cx, cy, r, Path.Direction.CW)

    private fun emoji() {
        circle(12f, 12f, 9f)
        circle(9f, 10f, 0.4f)
        circle(15f, 10f, 0.4f)
        oval.set(8f, 10.5f, 16f, 16.5f)
        path.arcTo(oval, 20f, 140f, true)
    }

    private fun keyboard() {
        oval.set(3f, 6f, 21f, 18f)
        path.addRoundRect(oval, 2.5f, 2.5f, Path.Direction.CW)
        for (y in listOf(9.5f, 12.2f)) {
            for (x in listOf(7f, 10.3f, 13.7f, 17f)) {
                path.moveTo(x - 0.3f, y)
                path.lineTo(x + 0.3f, y)
            }
        }
        path.moveTo(8.5f, 15f)
        path.lineTo(15.5f, 15f)
    }

    /** 一个「#」。 */
    private fun symbols() {
        polyline(10f, 4.5f, 8f, 19.5f)
        polyline(16f, 4.5f, 14f, 19.5f)
        polyline(5f, 9.5f, 19.5f, 9.5f)
        polyline(4.5f, 14.5f, 19f, 14.5f)
    }

    private fun resize() {
        polyline(12f, 3.5f, 12f, 20.5f)
        polyline(8f, 7.5f, 12f, 3.5f, 16f, 7.5f)
        polyline(8f, 16.5f, 12f, 20.5f, 16f, 16.5f)
    }

    /** 垃圾桶：盖子、提手与桶身两道竖纹。 */
    private fun trash() {
        polyline(4.5f, 7f, 19.5f, 7f)
        polyline(9.5f, 7f, 9.5f, 4.5f, 14.5f, 4.5f, 14.5f, 7f)
        polyline(6.5f, 7f, 7.5f, 20f, 16.5f, 20f, 17.5f, 7f)
        polyline(10.5f, 10.5f, 10.5f, 16.5f)
        polyline(13.5f, 10.5f, 13.5f, 16.5f)
    }

    /** 拆词：方框里一道竖线把字分开。 */
    private fun split() {
        oval.set(4f, 5f, 20f, 19f)
        path.addRoundRect(oval, 2f, 2f, Path.Direction.CW)
        polyline(12f, 7.5f, 12f, 16.5f)
        polyline(7f, 12f, 9.5f, 12f)
        polyline(14.5f, 12f, 17f, 12f)
    }

    /** 复制：两张错开的纸。 */
    private fun copy() {
        oval.set(8.5f, 8.5f, 19.5f, 19.5f)
        path.addRoundRect(oval, 2f, 2f, Path.Direction.CW)
        polyline(15.5f, 8.5f, 15.5f, 4.5f, 4.5f, 4.5f, 4.5f, 15.5f, 8.5f, 15.5f)
    }

    /** 夹板：板子加顶上的夹子，板上两行字。 */
    /** 翻译：左上一个「文」，右下一个「A」。 */
    private fun translate() {
        polyline(7.5f, 3f, 7.5f, 5.5f)
        polyline(3f, 5.5f, 12f, 5.5f)
        polyline(10.5f, 5.5f, 7.5f, 10.5f, 3.5f, 13.5f)
        polyline(5f, 8f, 8f, 11f, 11.5f, 13.5f)
        polyline(12.5f, 21f, 16.5f, 11f, 20.5f, 21f)
        polyline(14f, 17.5f, 19f, 17.5f)
    }

    private fun clipboard() {
        oval.set(5f, 5f, 19f, 21f)
        path.addRoundRect(oval, 2.5f, 2.5f, Path.Direction.CW)
        oval.set(9f, 3f, 15f, 7f)
        path.addRoundRect(oval, 1.2f, 1.2f, Path.Direction.CW)
        polyline(8.5f, 11.5f, 15.5f, 11.5f)
        polyline(8.5f, 15f, 13.5f, 15f)
    }

    /** 话筒：圆角胶囊、托架与底座。 */
    private fun microphone() {
        oval.set(9f, 3f, 15f, 14.5f)
        path.addRoundRect(oval, 3f, 3f, Path.Direction.CW)
        oval.set(5.5f, 5f, 18.5f, 17.5f)
        path.arcTo(oval, 0f, 180f, true)
        polyline(12f, 17.5f, 12f, 21f)
        polyline(8.5f, 21f, 15.5f, 21f)
    }

    /** 六边形螺母加中间一个圈，和系统设置的齿轮区分开。 */
    private fun settings() {
        val r = 8.5f
        for (i in 0..6) {
            val angle = Math.toRadians(30.0 + 60.0 * i)
            val x = 12f + r * kotlin.math.cos(angle).toFloat()
            val y = 12f + r * kotlin.math.sin(angle).toFloat()
            if (i == 0) path.moveTo(x, y) else path.lineTo(x, y)
        }
        circle(12f, 12f, 3f)
    }

    private fun globe() {
        circle(12f, 12f, 8.5f)
        oval.set(8.2f, 3.5f, 15.8f, 20.5f)
        path.addOval(oval, Path.Direction.CW)
        polyline(3.5f, 12f, 20.5f, 12f)
    }

    private fun home() {
        polyline(3.5f, 11f, 12f, 4f, 20.5f, 11f)
        polyline(6f, 9.5f, 6f, 19.5f, 18f, 19.5f, 18f, 9.5f)
        polyline(10f, 19.5f, 10f, 14.5f, 14f, 14.5f, 14f, 19.5f)
    }

    /** 一本打开的书。 */
    private fun book() {
        polyline(12f, 6.5f, 12f, 19.5f)
        polyline(12f, 6.5f, 8.5f, 5f, 3.5f, 5.5f, 3.5f, 18f, 8.5f, 17.5f, 12f, 19.5f)
        polyline(12f, 6.5f, 15.5f, 5f, 20.5f, 5.5f, 20.5f, 18f, 15.5f, 17.5f, 12f, 19.5f)
    }

    /** 三根高低不一的柱子立在底线上。 */
    private fun chart() {
        polyline(3.5f, 20f, 20.5f, 20f)
        polyline(7f, 20f, 7f, 13f)
        polyline(12f, 20f, 12f, 5.5f)
        polyline(17f, 20f, 17f, 10f)
    }

    // 平底云：左小鼓包、顶上大鼓包、右中鼓包，同候选条的小云朵
    private fun cloud() {
        path.moveTo(17f, 18f)
        path.lineTo(6.5f, 18f)
        oval.set(3f, 11f, 10f, 18f)
        path.arcTo(oval, 90f, 180f)
        oval.set(7f, 5f, 17f, 15f)
        path.arcTo(oval, 190f, 150f)
        oval.set(13f, 10f, 21f, 18f)
        path.arcTo(oval, 270f, 180f)
    }

    private fun user() {
        circle(12f, 8.5f, 4f)
        oval.set(4.5f, 14f, 19.5f, 26f)
        path.arcTo(oval, 200f, 140f, true)
    }

    /** 调色盘：右下缺一口的圆，三个色点。 */
    private fun palette() {
        oval.set(3.5f, 3.5f, 20.5f, 20.5f)
        path.addArc(oval, 60f, 300f)
        path.quadTo(14.5f, 13.5f, 16.5f, 16.5f)
        circle(8f, 10f, 1.2f)
        circle(12f, 7f, 1.2f)
        circle(16f, 10f, 1.2f)
    }

    private fun info() {
        circle(12f, 12f, 9f)
        polyline(12f, 11f, 12f, 16.5f)
        circle(12f, 7.8f, 0.3f)
    }

    private fun lock(open: Boolean) {
        oval.set(5.5f, 11f, 18.5f, 20f)
        path.addRoundRect(oval, 2f, 2f, Path.Direction.CW)
        oval.set(8.5f, 4f, 15.5f, 12f)
        if (open) {
            path.arcTo(oval, 180f, 160f, true)
        } else {
            path.moveTo(8.5f, 11f)
            path.lineTo(8.5f, 8f)
            path.arcTo(oval, 180f, 180f, false)
            path.lineTo(15.5f, 11f)
        }
        polyline(12f, 14.5f, 12f, 16.5f)
    }
}
