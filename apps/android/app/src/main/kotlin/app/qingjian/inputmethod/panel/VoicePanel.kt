package app.qingjian.inputmethod.panel

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RadialGradient
import android.graphics.RectF
import android.graphics.Shader
import android.view.MotionEvent
import android.view.View
import app.qingjian.inputmethod.keyboard.Haptics
import app.qingjian.inputmethod.panel.base.Ink
import app.qingjian.inputmethod.panel.base.PanelIcons.Icon
import app.qingjian.inputmethod.theme.Palette
import kotlin.math.abs
import kotlin.math.atan2
import kotlin.math.hypot

/**
 * 说话时盖住候选条与键盘的面板：底板上一团主题色的光，上方一条细波形，中间一段弧，
 * 左边「撤回输入」、右边是输入框的回车动作（发送、搜索…），下面一行提示、一个圆钮，最底一排 ⌄ 与回键盘。
 * 按着空格说时手指滑到哪块上松手就是哪个（[track]）；点按进来的各块都能点。听出来的字不在这里，直接进输入框。
 * 只管画与点，状态由 [app.qingjian.inputmethod.voice.VoiceInput] 推过来。
 */
@SuppressLint("ViewConstructor")
class VoicePanel(
    context: Context,
    private val palette: Palette,
    /** 右边那块弧上的字：输入框的回车动作，没有动作时是「完成」。 */
    private val actionLabel: String,
    private val actions: Actions,
) : View(context) {
    interface Actions {
        /** 撤回：这一段不要了。 */
        fun onCancel()

        /** 说完了；[act] 是点了右边那块（上屏后执行回车动作），[hide] 是 ⌄（上屏后收起键盘）。 */
        fun onFinish(act: Boolean, hide: Boolean)
    }

    /** 按着空格说时手指停在哪块上。 */
    enum class Target { NONE, CANCEL, ACTION }

    private val ink = Ink(palette, resources.displayMetrics.density)
    private val paint = Paint(Paint.ANTI_ALIAS_FLAG)
    private val glow = Paint(Paint.ANTI_ALIAS_FLAG)
    private val rect = RectF()
    private val location = IntArray(2)

    /** 按着空格说（松手结束）；否则是点按进来的。 */
    var holding = false
        set(value) {
            field = value
            invalidate()
        }

    /** 说完了，等最终结果。 */
    var recognizing = false
        set(value) {
            field = value
            invalidate()
        }

    var target = Target.NONE
        private set

    /** 最近的音量，新的在末尾；波形从右往左流。 */
    private val levels = FloatArray(BARS)

    init {
        setBackgroundColor(palette.board)
    }

    /** 新到一截录音的音量（0 到 1）：波形往左挪一格。 */
    fun addLevel(level: Float) {
        System.arraycopy(levels, 1, levels, 0, BARS - 1)
        levels[BARS - 1] = level.coerceIn(0f, 1f)
        invalidate()
    }

    /** 按着空格的手指挪到了屏幕上 ([screenX], [screenY])：落在哪块弧上就亮哪块。 */
    fun track(screenX: Float, screenY: Float) {
        getLocationOnScreen(location)
        val next = targetAt(screenX - location[0], screenY - location[1])
        if (next == target) return
        target = next
        if (next != Target.NONE) Haptics.tap(context)
        invalidate()
    }

    // 弧是一个大圆的顶上一段：圆心在面板下方，两块各占圆心角 [PAD_FROM, PAD_TO] 度（离正上方）
    private val radius get() = width * 1.0f
    private val arcTop get() = height * 0.5f
    private val centreY get() = arcTop + radius
    private val thickness get() = height * 0.18f

    private fun targetAt(x: Float, y: Float): Target {
        val distance = hypot(x - width / 2f, y - centreY)
        if (distance < radius - thickness / 2 - ink.dp(24f) || distance > radius + thickness / 2 + ink.dp(40f)) return Target.NONE
        // 离正上方的角度，左负右正
        val angle = Math.toDegrees(atan2((x - width / 2f).toDouble(), (centreY - y).toDouble())).toFloat()
        return when {
            angle <= -PAD_FROM + 2 && angle >= -PAD_TO - 20 -> Target.CANCEL
            angle >= PAD_FROM - 2 && angle <= PAD_TO + 20 -> Target.ACTION
            else -> Target.NONE
        }
    }

    private fun hint() = when {
        recognizing -> "正在识别…"
        !holding -> "说完点下面的话筒"
        target == Target.CANCEL -> "松手撤回"
        target == Target.ACTION -> "松手$actionLabel"
        else -> "松手结束"
    }

    override fun onDraw(canvas: Canvas) {
        val w = width.toFloat()
        val h = height.toFloat()
        glow.shader = RadialGradient(
            w / 2, h * 0.55f, w * 0.6f,
            withAlpha(palette.brand, 0x38), withAlpha(palette.brand, 0),
            Shader.TileMode.CLAMP,
        )
        canvas.drawRect(0f, 0f, w, h, glow)

        drawWave(canvas, h * 0.17f)
        drawPad(canvas, Target.CANCEL, "撤回输入")
        drawPad(canvas, Target.ACTION, actionLabel)
        ink.text(canvas, hint(), 13f, palette.gloss, w / 2, arcTop + thickness / 2 + h * 0.09f)

        // 圆钮：按着说时是手指的位置提示，点按进来时是「说完了」
        val buttonY = h * 0.8f
        val buttonRadius = ink.dp(if (holding) 18f else 24f)
        paint.style = Paint.Style.FILL
        paint.color = palette.surface
        canvas.drawCircle(w / 2, buttonY, buttonRadius, paint)
        paint.style = Paint.Style.STROKE
        paint.strokeWidth = ink.dp(1.5f)
        paint.color = withAlpha(palette.brand, 0x80)
        canvas.drawCircle(w / 2, buttonY, buttonRadius, paint)
        paint.style = Paint.Style.FILL
        if (!holding) ink.icon(canvas, Icon.MIC, w / 2, buttonY, 22f, palette.brand)

        val rowY = h - ink.dp(22f)
        ink.icon(canvas, Icon.HIDE, ink.dp(40f), rowY, 22f, palette.gloss)
        ink.icon(canvas, Icon.KEYBOARD, w - ink.dp(40f), rowY, 24f, palette.gloss)
    }

    /** 弧上的一块：粗弧线当底，字沿弧摆正（转到这块中间的切线方向）。 */
    private fun drawPad(canvas: Canvas, which: Target, label: String) {
        val sign = if (which == Target.CANCEL) -1f else 1f
        val active = target == which
        val cx = width / 2f
        rect.set(cx - radius, centreY - radius, cx + radius, centreY + radius)
        paint.style = Paint.Style.STROKE
        paint.strokeWidth = thickness
        paint.strokeCap = Paint.Cap.ROUND
        paint.color = if (active) palette.brand else withAlpha(palette.functionalKey, 0xA0)
        // drawArc 的角度从 3 点钟顺时针量；正上方是 -90
        val start = if (sign < 0) -90f - PAD_TO else -90f + PAD_FROM
        canvas.drawArc(rect, start, PAD_TO - PAD_FROM, false, paint)
        paint.style = Paint.Style.FILL
        paint.strokeCap = Paint.Cap.BUTT

        val middle = sign * (PAD_FROM + PAD_TO) / 2
        val radians = Math.toRadians(middle.toDouble())
        val tx = cx + radius * Math.sin(radians).toFloat()
        val ty = centreY - radius * Math.cos(radians).toFloat()
        canvas.save()
        canvas.rotate(middle, tx, ty)
        ink.text(canvas, label, 16f, if (active) palette.onBrand else palette.text, tx, ty, bold = active)
        canvas.restore()
    }

    /** 一排细竖条，高度按最近的音量；等结果时压成一排点。 */
    private fun drawWave(canvas: Canvas, cy: Float) {
        val span = width * 0.42f
        val left = (width - span) / 2
        val step = span / BARS
        val barWidth = ink.dp(3f)
        val maxHalf = ink.dp(18f)
        for (i in 0 until BARS) {
            // 两头淡、中间浓
            val edge = 1f - abs(i - (BARS - 1) / 2f) / BARS
            val level = if (recognizing) 0f else levels[i]
            val half = barWidth / 2 + (maxHalf - barWidth / 2) * level * edge
            val x = left + step * (i + 0.5f)
            paint.color = withAlpha(palette.brand, (140 + 115 * edge).toInt().coerceAtMost(255))
            rect.set(x - barWidth / 2, cy - half, x + barWidth / 2, cy + half)
            canvas.drawRoundRect(rect, barWidth / 2, barWidth / 2, paint)
        }
    }

    @SuppressLint("ClickableViewAccessibility")
    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (event.actionMasked != MotionEvent.ACTION_UP || holding || recognizing) return true
        val x = event.x
        val y = event.y
        val rowY = height - ink.dp(22f)
        val buttonY = height * 0.8f
        val near = ink.dp(32f)
        val tapped = when {
            targetAt(x, y) == Target.CANCEL -> { actions.onCancel(); true }
            targetAt(x, y) == Target.ACTION -> { actions.onFinish(act = true, hide = false); true }
            hypot(x - width / 2f, y - buttonY) < near -> { actions.onFinish(act = false, hide = false); true }
            y > rowY - near && x < ink.dp(80f) -> { actions.onFinish(act = false, hide = true); true }
            y > rowY - near && x > width - ink.dp(80f) -> { actions.onFinish(act = false, hide = false); true }
            else -> false
        }
        if (tapped) Haptics.tap(context)
        return true
    }

    private fun withAlpha(color: Int, alpha: Int) = (color and 0xFFFFFF) or (alpha shl 24)

    private companion object {
        const val BARS = 32

        /** 两块弧离正上方的角度范围（度）。 */
        const val PAD_FROM = 4f
        const val PAD_TO = 28f
    }
}
