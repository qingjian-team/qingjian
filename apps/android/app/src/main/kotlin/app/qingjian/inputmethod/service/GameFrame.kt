package app.qingjian.inputmethod.service

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.Outline
import android.graphics.Paint
import android.graphics.Rect
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.view.ViewOutlineProvider
import android.widget.FrameLayout
import android.widget.LinearLayout
import app.qingjian.inputmethod.panel.base.PanelIcons
import app.qingjian.inputmethod.panel.base.PanelIcons.Icon
import app.qingjian.inputmethod.theme.Palette

/**
 * 游戏键盘：铺满整屏的透明底，中间浮一块可拖动的键盘（顶上一条拖动栏 + [content]，即候选条与键盘）。
 * 整屏只有键盘那一块收触摸，其余透给下面的游戏，见服务的 `onComputeInsets` 与 [touchableRect]。
 * 位置记成左上角在可挪范围里的比例（[positionX] / [positionY]），换了屏幕尺寸、键盘大小也落在屏内。
 * 底边留出 [BOTTOM_GUARD_DP]：贴底时最下一排键会落进系统底部手势的感应区（MIUI 上点空格、符号被当成手势吃掉）。
 */
@SuppressLint("ViewConstructor")
class GameFrame(
    context: Context,
    palette: Palette,
    content: View,
    private val widthFraction: Float,
    transparency: Int,
    var positionX: Float,
    var positionY: Float,
    private val listener: Listener,
) : ViewGroup(context) {
    interface Listener {
        /** 拖完一次，位置定在 ([x], [y])。 */
        fun onGameMoved(x: Float, y: Float)

        /** 点了透明度按钮：换下一档。 */
        fun onGameTransparency()

        /** 点了键盘按钮：这个应用换回普通键盘。 */
        fun onGameNormalKeyboard()

        fun onGameHide()
    }

    private val density = resources.displayMetrics.density
    private fun dp(value: Float) = (value * density).toInt()

    private val header = Header(context, palette)
    private val origin = IntArray(2)

    private val box = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        background = GradientDrawable().apply {
            setColor(palette.board)
            cornerRadius = RADIUS_DP * density
        }
        outlineProvider = object : ViewOutlineProvider() {
            override fun getOutline(view: View, outline: Outline) =
                outline.setRoundRect(0, 0, view.width, view.height, RADIUS_DP * density)
        }
        clipToOutline = true
        elevation = 8 * density
        addView(header, LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(HEADER_DP)))
        addView(content, LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT))
    }

    init {
        addView(box)
        setTransparency(transparency)
    }

    fun setTransparency(percent: Int) {
        box.alpha = 1f - percent / 100f
    }

    /** 键盘那一块在窗口里的位置，交给 `onComputeInsets` 当可触摸区域。 */
    fun touchableRect(out: Rect) {
        getLocationInWindow(origin)
        out.set(origin[0] + box.left, origin[1] + box.top, origin[0] + box.right, origin[1] + box.bottom)
    }

    // 输入法窗口给的高度是整屏；铺满它，键盘才能拖到任意高度
    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val width = MeasureSpec.getSize(widthMeasureSpec)
        val height = MeasureSpec.getSize(heightMeasureSpec)
        box.measure(
            MeasureSpec.makeMeasureSpec((width * widthFraction).toInt(), MeasureSpec.EXACTLY),
            MeasureSpec.makeMeasureSpec(height, MeasureSpec.AT_MOST),
        )
        setMeasuredDimension(width, height)
    }

    override fun onLayout(changed: Boolean, l: Int, t: Int, r: Int, b: Int) {
        val x = ((width - box.measuredWidth).coerceAtLeast(0) * positionX).toInt()
        val y = (freeHeight(box.measuredHeight) * positionY).toInt()
        box.layout(x, y, x + box.measuredWidth, y + box.measuredHeight)
    }

    /** 键盘上下能挪的范围：底边让出手势区。 */
    private fun freeHeight(boxHeight: Int) = (height - dp(BOTTOM_GUARD_DP) - boxHeight).coerceAtLeast(0)

    /** 拖动栏：左边透明度、换回普通键盘，中间一道拖动条，右边收起；空白处按住拖动整块键盘。 */
    @SuppressLint("ViewConstructor")
    private inner class Header(context: Context, palette: Palette) : FrameLayout(context) {
        private val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = palette.text }
        private val grip = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = palette.faint }
        private var downX = 0f
        private var downY = 0f
        private var fromX = 0f
        private var fromY = 0f

        init {
            setWillNotDraw(false)
            val left = LinearLayout(context).apply {
                addView(button(null) { listener.onGameTransparency() })
                addView(button(Icon.KEYBOARD) { listener.onGameNormalKeyboard() })
            }
            addView(left, LayoutParams(LayoutParams.WRAP_CONTENT, LayoutParams.MATCH_PARENT, Gravity.START))
            addView(
                button(Icon.HIDE) { listener.onGameHide() },
                LayoutParams(dp(BUTTON_DP), LayoutParams.MATCH_PARENT, Gravity.END),
            )
        }

        /** [icon] 为 null 画透明度的半实心圆。 */
        private fun button(icon: Icon?, onClick: () -> Unit) = object : View(context) {
            override fun onDraw(canvas: Canvas) {
                val cx = width / 2f
                val cy = height / 2f
                if (icon != null) return PanelIcons.draw(canvas, icon, cx, cy, ICON_DP * density, paint)
                val r = 6.5f * density
                paint.style = Paint.Style.STROKE
                paint.strokeWidth = 1.6f * density
                canvas.drawCircle(cx, cy, r, paint)
                paint.style = Paint.Style.FILL
                canvas.drawArc(cx - r, cy - r, cx + r, cy + r, -90f, 180f, true, paint)
            }
        }.apply {
            layoutParams = LinearLayout.LayoutParams(dp(BUTTON_DP), LinearLayout.LayoutParams.MATCH_PARENT)
            setOnClickListener { onClick() }
        }

        override fun onDraw(canvas: Canvas) {
            val w = GRIP_WIDTH_DP * density
            val h = 4 * density
            canvas.drawRoundRect(
                (width - w) / 2, (height - h) / 2, (width + w) / 2, (height + h) / 2, h / 2, h / 2, grip,
            )
        }

        @SuppressLint("ClickableViewAccessibility")
        override fun onTouchEvent(event: MotionEvent): Boolean {
            val frame = this@GameFrame
            val freeX = (frame.width - box.width).coerceAtLeast(1).toFloat()
            val freeY = freeHeight(box.height).coerceAtLeast(1).toFloat()
            when (event.actionMasked) {
                MotionEvent.ACTION_DOWN -> {
                    downX = event.rawX
                    downY = event.rawY
                    fromX = positionX
                    fromY = positionY
                }
                MotionEvent.ACTION_MOVE -> {
                    positionX = (fromX + (event.rawX - downX) / freeX).coerceIn(0f, 1f)
                    positionY = (fromY + (event.rawY - downY) / freeY).coerceIn(0f, 1f)
                    frame.requestLayout()
                }
                MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> listener.onGameMoved(positionX, positionY)
            }
            return true
        }
    }

    private companion object {
        const val RADIUS_DP = 14f
        const val HEADER_DP = 28f
        const val BUTTON_DP = 34f
        const val ICON_DP = 18f
        const val GRIP_WIDTH_DP = 44f
        const val BOTTOM_GUARD_DP = 32f
    }
}
