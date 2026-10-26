package app.qingjian.inputmethod.keyboard

import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path
import app.qingjian.inputmethod.keyboard.layout.Key
import app.qingjian.inputmethod.keyboard.layout.KeySlot
import app.qingjian.inputmethod.theme.Palette
import kotlin.math.hypot
import org.json.JSONArray
import org.json.JSONObject

/**
 * 一笔滑行：手指落在字母键上就开始记轨迹，挪开 [START_KEYS] 个键宽才算滑行（之前照常是点按），之后画一道主题色的线。
 * 抬手时连同各字母键的中心与大小拼成交给 Rust 解码的 JSON（`{points, keys}`，像素）；判定与画在这里，解码在 Core。
 */
class SwipeTrail(palette: Palette, density: Float) {
    /** 在记的那根手指；-1 是没在记。 */
    var pointer = -1
        private set

    /** 已经挪开够远，这一笔是滑行，不再当点按。 */
    var swiping = false
        private set

    private val xs = ArrayList<Float>()
    private val ys = ArrayList<Float>()
    private var threshold = 0f
    private val path = Path()
    private val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        strokeWidth = density * STROKE_DP
        strokeCap = Paint.Cap.ROUND
        strokeJoin = Paint.Join.ROUND
        color = (palette.brand and 0xFFFFFF) or (ALPHA shl 24)
    }

    /** 手指 [pointer] 落在一个宽 [keyWidth] 的字母键上的 ([x], [y])。 */
    fun begin(pointer: Int, x: Float, y: Float, keyWidth: Float) {
        clear()
        this.pointer = pointer
        threshold = keyWidth * START_KEYS
        add(x, y)
    }

    /** 记一个点；返回这一点是不是刚好让它变成滑行（调用方据此撤掉点按的状态）。 */
    fun add(x: Float, y: Float): Boolean {
        if (pointer < 0) return false
        xs += x
        ys += y
        if (xs.size == 1) path.moveTo(x, y) else path.lineTo(x, y)
        if (swiping || hypot(x - xs[0], y - ys[0]) < threshold) return false
        swiping = true
        return true
    }

    fun clear() {
        pointer = -1
        swiping = false
        xs.clear()
        ys.clear()
        path.reset()
    }

    fun draw(canvas: Canvas) {
        if (swiping) canvas.drawPath(path, paint)
    }

    /** 轨迹与字母键的几何（[slots] 里的 [Key.Letter]），交 `Native.swipe`。 */
    fun toJson(slots: List<KeySlot>): String {
        val points = JSONArray()
        for (i in xs.indices) points.put(JSONObject().put("x", xs[i].toDouble()).put("y", ys[i].toDouble()))
        val keys = JSONArray()
        for (slot in slots) {
            val key = slot.key as? Key.Letter ?: continue
            keys.put(
                JSONObject()
                    .put("letter", key.char.toString())
                    .put("x", slot.bounds.centerX().toDouble())
                    .put("y", slot.bounds.centerY().toDouble())
                    .put("width", slot.bounds.width().toDouble())
                    .put("height", slot.bounds.height().toDouble()),
            )
        }
        return JSONObject().put("points", points).put("keys", keys).toString()
    }

    private companion object {
        /** 挪开多少个键宽算滑行：再小点按时手指一抖就误判，再大短词（相邻两键）滑不出来。 */
        const val START_KEYS = 0.6f
        const val STROKE_DP = 5f
        const val ALPHA = 0xB0
    }
}
