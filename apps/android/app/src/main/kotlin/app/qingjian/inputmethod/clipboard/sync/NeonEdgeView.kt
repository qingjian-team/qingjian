package app.qingjian.inputmethod.clipboard.sync

import android.animation.ArgbEvaluator
import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.LinearGradient
import android.graphics.Paint
import android.graphics.Shader
import android.os.SystemClock
import android.provider.Settings as SystemSettings
import android.view.View
import kotlin.math.PI
import kotlin.math.cos

/**
 * 键盘边缘的同步霓虹灯：收内容时流动，颜色随速度变，结束转绿或转红再收起。盖在键盘上，不接触摸。
 * 颜色和节奏与 Windows（apps/clipboard/src/system/neon/mod.rs）一致，改了要两边一起改。
 */
class NeonEdgeView(context: Context) : View(context) {
    private enum class Phase { IDLE, FLOWING, CLOSING }

    private val paint = Paint(Paint.ANTI_ALIAS_FLAG)
    private val evaluator = ArgbEvaluator()
    private val thickness = 14 * resources.displayMetrics.density
    private var phase = Phase.IDLE
    private var startedAt = 0L
    private var closingAt = 0L
    private var heardAt = 0L
    private var color = BLUE
    private var target = BLUE
    private var result = GREEN
    private var pendingResult: Int? = null

    /** 系统把动画关了就不流动，只亮和变色。 */
    private val animated: Boolean
        get() = SystemSettings.Global.getFloat(context.contentResolver, SystemSettings.Global.ANIMATOR_DURATION_SCALE, 1f) > 0f

    init {
        isClickable = false
        isFocusable = false
        importantForAccessibility = IMPORTANT_FOR_ACCESSIBILITY_NO
    }

    fun start() {
        val now = SystemClock.uptimeMillis()
        heardAt = now
        if (phase == Phase.FLOWING) return
        phase = Phase.FLOWING
        startedAt = now
        color = BLUE
        target = BLUE
        pendingResult = null
        postInvalidateOnAnimation()
    }

    fun speed(bytesPerSecond: Long) {
        heardAt = SystemClock.uptimeMillis()
        if (bytesPerSecond > 0) {
            target = when {
                bytesPerSecond >= FAST_BYTES -> VIOLET
                bytesPerSecond >= SLOW_BYTES -> CYAN
                else -> BLUE
            }
        }
    }

    fun finish(success: Boolean) {
        val outcome = if (success) GREEN else RED
        if (phase == Phase.IDLE) {
            // 发送失败之前没有 start，直接亮一下
            start()
            startedAt = SystemClock.uptimeMillis() - MIN_FLOW_MS
        }
        pendingResult = outcome
        postInvalidateOnAnimation()
    }

    override fun onDraw(canvas: Canvas) {
        if (phase == Phase.IDLE) return
        val now = SystemClock.uptimeMillis()
        if (phase == Phase.FLOWING && pendingResult == null && now - heardAt > STALL_LIMIT_MS) pendingResult = RED
        val outcome = pendingResult
        if (phase == Phase.FLOWING && outcome != null && now - startedAt >= MIN_FLOW_MS) {
            phase = Phase.CLOSING
            closingAt = now
            result = outcome
        }
        val strength = when (phase) {
            Phase.CLOSING -> {
                val progress = (now - closingAt).toFloat() / CLOSE_MS
                if (progress >= 1f) {
                    phase = Phase.IDLE
                    return
                }
                color = evaluator.evaluate((progress * 3f).coerceAtMost(1f), color, result) as Int
                1f - ((progress - 0.5f).coerceAtLeast(0f) * 2f).let { it * it }
            }
            else -> {
                color = evaluator.evaluate(0.12f, color, target) as Int
                ((now - startedAt) / 200f).coerceAtMost(1f)
            }
        }
        val time = if (animated) (now - startedAt) / 1000f else 0f
        drawEdges(canvas, strength, time)
        postInvalidateOnAnimation()
    }

    private fun drawEdges(canvas: Canvas, strength: Float, time: Float) {
        val w = width.toFloat()
        val h = height.toFloat()
        val perimeter = 2 * (w + h)
        var along = 0f
        val pieces = (w + h) / SEGMENT_PX
        // 把一圈切成小段，每段亮度跟着波走；一段内用向内的渐变
        val steps = pieces.toInt().coerceAtLeast(8)
        for (i in 0 until steps) {
            val x = w * i / steps
            val y = h * i / steps
            val xNext = w * (i + 1) / steps
            val yNext = h * (i + 1) / steps
            segment(canvas, x, 0f, xNext, thickness, 0f, 0f, 0f, thickness, wave(x / perimeter, time), strength)
            segment(canvas, x, h - thickness, xNext, h, 0f, h, 0f, h - thickness, wave((w + h + w - x) / perimeter, time), strength)
            segment(canvas, 0f, y, thickness, yNext, 0f, 0f, thickness, 0f, wave((2 * w + h + h - y) / perimeter, time), strength)
            segment(canvas, w - thickness, y, w, yNext, w, 0f, w - thickness, 0f, wave((w + y) / perimeter, time), strength)
            along += SEGMENT_PX
        }
    }

    private fun wave(phase: Float, time: Float) =
        0.62f + 0.38f * cos((phase * 3f - time * 0.7f) * 2f * PI.toFloat())

    private fun segment(
        canvas: Canvas, left: Float, top: Float, right: Float, bottom: Float,
        fromX: Float, fromY: Float, toX: Float, toY: Float, wave: Float, strength: Float,
    ) {
        val alpha = (255 * wave * strength).toInt().coerceIn(0, 255)
        val solid = Color.argb(alpha, Color.red(color), Color.green(color), Color.blue(color))
        paint.shader = LinearGradient(fromX, fromY, toX, toY, solid, solid and 0x00FFFFFF, Shader.TileMode.CLAMP)
        canvas.drawRect(left, top, right, bottom, paint)
    }

    private companion object {
        const val MIN_FLOW_MS = 600L
        const val CLOSE_MS = 550f
        const val STALL_LIMIT_MS = 20_000L
        const val SEGMENT_PX = 24f
        const val SLOW_BYTES = 1024L * 1024
        const val FAST_BYTES = 10L * 1024 * 1024
        val BLUE = Color.rgb(64, 140, 255)
        val CYAN = Color.rgb(40, 220, 235)
        val VIOLET = Color.rgb(175, 95, 255)
        val GREEN = Color.rgb(55, 225, 120)
        val RED = Color.rgb(255, 70, 70)
    }
}
