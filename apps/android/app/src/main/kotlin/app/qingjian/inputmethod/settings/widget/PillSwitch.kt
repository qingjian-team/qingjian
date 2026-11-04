package app.qingjian.inputmethod.settings.widget

import android.animation.ValueAnimator
import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import android.view.View
import android.view.accessibility.AccessibilityNodeInfo
import android.widget.Checkable

/**
 * 圆角轨道加白色圆钮的开关：开着轨道是 [on] 色、钮在右，关着是 [off] 色、钮在左，切换时钮滑过去。
 * 系统 Switch 换上自定义轨道后量出来的宽度不对、会被裁掉一截，所以自己画。
 */
class PillSwitch(context: Context, private val on: Int, private val off: Int) : View(context), Checkable {
    private val density = resources.displayMetrics.density
    private val paint = Paint(Paint.ANTI_ALIAS_FLAG)
    private val rect = RectF()

    /** 钮的位置：0 在左（关），1 在右（开）。 */
    private var position = 0f
    private var checked = false
    private var animator: ValueAnimator? = null

    var onChange: ((Boolean) -> Unit)? = null

    init {
        isClickable = true
        isFocusable = true
        setOnClickListener { toggle() }
    }

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) =
        setMeasuredDimension((WIDTH_DP * density).toInt(), (HEIGHT_DP * density).toInt())

    override fun isChecked() = checked

    override fun setChecked(value: Boolean) = set(value, animate = false)

    override fun toggle() = set(!checked, animate = true)

    private fun set(value: Boolean, animate: Boolean) {
        if (value == checked && animator == null) {
            position = if (value) 1f else 0f
            invalidate()
            return
        }
        val changed = value != checked
        checked = value
        animator?.cancel()
        val target = if (value) 1f else 0f
        if (animate && isAttachedToWindow) {
            animator = ValueAnimator.ofFloat(position, target).apply {
                duration = ANIMATION_MS
                addUpdateListener {
                    position = it.animatedValue as Float
                    invalidate()
                }
                start()
            }
        } else {
            position = target
            invalidate()
        }
        if (changed) onChange?.invoke(value)
    }

    override fun onDraw(canvas: Canvas) {
        val h = height.toFloat()
        val w = width.toFloat()
        paint.color = blend(off, on, position)
        rect.set(0f, 0f, w, h)
        canvas.drawRoundRect(rect, h / 2, h / 2, paint)
        val inset = 2 * density
        val radius = h / 2 - inset
        val x = inset + radius + (w - 2 * (inset + radius)) * position
        paint.color = SHADOW
        canvas.drawCircle(x, h / 2 + density * 0.8f, radius, paint)
        paint.color = KNOB
        canvas.drawCircle(x, h / 2, radius, paint)
    }

    override fun onInitializeAccessibilityNodeInfo(info: AccessibilityNodeInfo) {
        super.onInitializeAccessibilityNodeInfo(info)
        info.className = "android.widget.Switch"
        info.isCheckable = true
        info.isChecked = checked
    }

    private fun blend(from: Int, to: Int, t: Float): Int {
        fun channel(shift: Int) = ((from shr shift and 0xFF) + ((to shr shift and 0xFF) - (from shr shift and 0xFF)) * t).toInt()
        return (channel(24) shl 24) or (channel(16) shl 16) or (channel(8) shl 8) or channel(0)
    }

    companion object {
        private const val WIDTH_DP = 50
        private const val HEIGHT_DP = 30
        private const val ANIMATION_MS = 180L
        private const val KNOB = 0xFFFFFFFF.toInt()
        private const val SHADOW = 0x22000000
    }
}
