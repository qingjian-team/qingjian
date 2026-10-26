package app.qingjian.inputmethod.keyboard

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.view.MotionEvent
import android.view.View
import app.qingjian.inputmethod.panel.base.PanelIcons
import app.qingjian.inputmethod.settings.BottomAction
import app.qingjian.inputmethod.theme.Palette

/**
 * 全面屏适配：键盘下面、手势条所在的那一截不留白，左右两个角各放一个键，放什么在键盘设置里选。
 * [inset] 是系统导航栏 / 手势条的高度，图标画在它上面的那一段里。
 */
@SuppressLint("ViewConstructor")
class BottomActionsView(
    context: Context,
    palette: Palette,
    private val inset: Int,
    private val left: BottomAction,
    private val right: BottomAction,
    private val onAction: (BottomAction) -> Unit,
) : View(context) {
    private val density = resources.displayMetrics.density
    private val stripHeight = 40f * density
    private val slotWidth = 72f * density
    private val stroke = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = palette.gloss }

    init {
        setBackgroundColor(palette.board)
    }

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        setMeasuredDimension(MeasureSpec.getSize(widthMeasureSpec), (stripHeight + inset).toInt())
    }

    override fun onDraw(canvas: Canvas) {
        val cy = stripHeight / 2 + 2 * density
        left.icon?.let { PanelIcons.draw(canvas, it, slotWidth / 2, cy, 24 * density, stroke) }
        right.icon?.let { PanelIcons.draw(canvas, it, width - slotWidth / 2, cy, 24 * density, stroke) }
    }

    @SuppressLint("ClickableViewAccessibility")
    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (event.actionMasked == MotionEvent.ACTION_UP) {
            when {
                event.x < slotWidth -> onAction(left)
                event.x > width - slotWidth -> onAction(right)
            }
        }
        return true
    }
}
