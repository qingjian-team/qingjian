package app.qingjian.inputmethod.candidates

import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import android.text.TextPaint
import android.view.MotionEvent
import app.qingjian.inputmethod.theme.Palette
import kotlin.math.abs

/**
 * 调高度模式下候选条变成的拖动手柄：上下拖改键盘高度，点「完成」退出。
 * 位移按屏幕坐标算——键盘变高时这条自己也在动。
 */
class ResizeHandle(
    private val palette: Palette,
    private val density: Float,
    private val barHeight: Float,
    private val onDrag: (deltaPx: Float, done: Boolean) -> Unit,
    private val onDone: () -> Unit,
) {
    private val fill = Paint(Paint.ANTI_ALIAS_FLAG)
    private val label = TextPaint(Paint.ANTI_ALIAS_FLAG).also { it.textAlign = Paint.Align.CENTER }
    private val rect = RectF()
    private var startY = 0f
    private var dragged = false

    fun draw(canvas: Canvas, viewWidth: Int) {
        val cx = viewWidth / 2f
        fill.color = palette.faint
        rect.set(cx - 22 * density, 10 * density, cx + 22 * density, 15 * density)
        canvas.drawRoundRect(rect, 3 * density, 3 * density, fill)
        label.textSize = 11.5f * density
        label.color = palette.gloss
        canvas.drawText("上下拖动调整键盘高度", cx, barHeight / 2 + 12 * density, label)

        doneBounds(viewWidth)
        fill.color = palette.brand
        canvas.drawRoundRect(rect, 15 * density, 15 * density, fill)
        label.textSize = 14f * density
        label.color = palette.onBrand
        canvas.drawText("完成", rect.centerX(), rect.centerY() + 5 * density, label)
    }

    fun onTouch(event: MotionEvent, viewWidth: Int): Boolean {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                startY = event.rawY
                dragged = false
            }

            MotionEvent.ACTION_MOVE -> {
                val delta = startY - event.rawY
                if (dragged || abs(delta) > 6 * density) {
                    dragged = true
                    onDrag(delta, false)
                }
            }

            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> when {
                dragged -> onDrag(startY - event.rawY, true)
                doneBounds(viewWidth).contains(event.x, event.y) -> onDone()
            }
        }
        return true
    }

    private fun doneBounds(viewWidth: Int): RectF = rect.apply {
        set(
            viewWidth - 76 * density, barHeight / 2 - 15 * density,
            viewWidth - 12 * density, barHeight / 2 + 15 * density,
        )
    }
}
