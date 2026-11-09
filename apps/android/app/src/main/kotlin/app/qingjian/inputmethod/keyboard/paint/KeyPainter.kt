package app.qingjian.inputmethod.keyboard.paint

import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import app.qingjian.inputmethod.keyboard.ShiftState
import app.qingjian.inputmethod.keyboard.SideColumn
import app.qingjian.inputmethod.keyboard.layout.Key
import app.qingjian.inputmethod.keyboard.layout.KeySlot
import app.qingjian.inputmethod.keyboard.layout.KeyboardMetrics
import app.qingjian.inputmethod.theme.Palette

/** 画键与按键气泡。 */
class KeyPainter(
    private val palette: Palette,
    private val metrics: KeyboardMetrics,
    /** 键面字号档的倍数（键盘设置「键面字号」），键上的主字与气泡一起缩放。 */
    private val textScale: Float,
) {
    private val density = metrics.density
    private val fill = Paint(Paint.ANTI_ALIAS_FLAG)
    private val ink = Paint(Paint.ANTI_ALIAS_FLAG).apply { textAlign = Paint.Align.CENTER }
    private val bubble = RectF()

    var shift = ShiftState.OFF
    var chinese = true

    /** 中英键上的字（当前语言）与角上的小字（按一下切到的那种）。 */
    var languageLabel = "中"
    var languageHint = ""

    /** 回车键上的字（发送、搜索…）；null 画回车图标。 */
    var enterLabel: String? = null

    fun drawKey(canvas: Canvas, slot: KeySlot, down: Boolean) {
        val key = slot.key
        val bounds = slot.bounds
        val brand = key is Key.Enter
        drawPlate(
            canvas, bounds,
            when {
                brand -> palette.brand
                down || ((key == Key.Shift || (key as? Key.Letter)?.flickShift == true) && shift.on) -> palette.accent
                key.functional -> palette.functionalKey
                else -> palette.surface
            },
        )
        if (brand && down) {
            fill.color = PRESSED_BRAND_SHADE
            canvas.drawRoundRect(bounds, metrics.radius, metrics.radius, fill)
        }

        // 锁定大写：在亮着的底色上再加右上角一个品牌色小点，和「只大写一次」分开
        if (shift == ShiftState.LOCKED && (key == Key.Shift || (key as? Key.Letter)?.flickShift == true)) {
            fill.color = palette.brand
            canvas.drawCircle(bounds.right - LOCK_DOT_INSET_DP * density, bounds.top + LOCK_DOT_INSET_DP * density, LOCK_DOT_DP * density, fill)
        }

        // 角标先画会把 ink 换成浅色，正文的颜色另存一份
        val labelColor = if (brand) palette.onBrand else palette.text
        ink.color = labelColor
        val label = label(key)
        when {
            label.isEmpty() ->
                KeyIcons.draw(canvas, key, bounds.centerX(), bounds.centerY(), 24 * density, ink, shift)

            // 兼作大写的 `;`：上方小 Shift 图标（上滑出它），中间 `;`，下方韵母
            key is Key.Letter && key.flickShift -> {
                ink.color = palette.gloss
                KeyIcons.draw(canvas, Key.Shift, bounds.centerX(), bounds.top + bounds.height() * 0.22f, 13 * density, ink, shift)
                text(canvas, label, 19f, labelColor, bounds.centerX(), bounds.top + bounds.height() * 0.5f)
                fitted(canvas, key.finals, 9f, palette.gloss, bounds.centerX(), bounds.top + bounds.height() * 0.8f, bounds.width() * 0.92f)
            }

            // 双拼：上方翘舌声母（没有就印角标），中间字母，下方韵母
            key is Key.Letter && (key.initial.isNotEmpty() || key.finals.isNotEmpty()) -> {
                val top = key.initial.ifEmpty { key.hint }
                val topColor = if (key.initial.isNotEmpty()) palette.brand else palette.gloss
                text(canvas, top, 10f, topColor, bounds.centerX(), bounds.top + bounds.height() * 0.2f)
                text(canvas, label, 19f, labelColor, bounds.centerX(), bounds.top + bounds.height() * 0.5f)
                fitted(canvas, key.finals, 9f, palette.gloss, bounds.centerX(), bounds.top + bounds.height() * 0.8f, bounds.width() * 0.92f)
            }

            key is Key.Letter && key.hint.isNotEmpty() -> {
                text(canvas, key.hint, 11f, palette.gloss, bounds.centerX(), bounds.top + bounds.height() * 0.24f)
                text(canvas, label, 21f, labelColor, bounds.centerX(), bounds.top + bounds.height() * belowHint)
            }

            // 中英键：角上写按一下会切到哪种语言，长按换
            key == Key.Language && languageHint.isNotEmpty() -> {
                text(canvas, languageHint, 10f, palette.gloss, bounds.right - bounds.width() * 0.22f, bounds.top + bounds.height() * 0.24f)
                text(canvas, label, 17f, labelColor, bounds.centerX(), bounds.centerY())
            }

            key is Key.Digit -> {
                text(canvas, key.digit.toString(), 11f, palette.faint, bounds.centerX(), bounds.top + bounds.height() * 0.24f)
                text(canvas, label, 18f, palette.text, bounds.centerX(), bounds.top + bounds.height() * belowHint)
            }

            else -> {
                val size = when (key) {
                    is Key.Letter -> 22f
                    is Key.Zhuyin -> 20f
                    is Key.Text -> 20f
                    is Key.Space -> 13f
                    else -> 15f
                }
                text(canvas, label, size, if (key is Key.Space) palette.faint else ink.color, bounds.centerX(), bounds.centerY())
            }
        }
    }

    /** 按下的键上方放大显示 [text]（缺省是键面的字，上滑时是要出的那个）；顶排的气泡会画到候选条上。 */
    fun drawBubble(canvas: Canvas, slot: KeySlot, viewWidth: Int, text: String = label(slot.key)) {
        val bounds = slot.bounds
        val grow = 7f * density
        bubble.set(
            (bounds.left - grow).coerceAtLeast(metrics.gapX),
            bounds.top - metrics.rowHeight - 2 * density,
            (bounds.right + grow).coerceAtMost(viewWidth - metrics.gapX),
            bounds.top - 4 * density,
        )
        drawPlate(canvas, bubble, palette.surface)
        text(canvas, text, 30f, palette.text, bubble.centerX(), bubble.centerY())
    }

    /** 拼音列：一块圆角面板，项目竖排。 */
    fun drawSide(canvas: Canvas, side: SideColumn) {
        fill.color = palette.functionalKey
        canvas.drawRoundRect(side.bounds, metrics.radius, metrics.radius, fill)
        canvas.save()
        canvas.clipRect(side.bounds)
        side.visibleItems { item, cy -> text(canvas, item, 17f, palette.text, side.bounds.centerX(), cy) }
        canvas.restore()
    }

    private fun label(key: Key) = when (key) {
        // 中文键盘一律画大写；英文跟着 Shift
        is Key.Letter -> if (shift.on || chinese) key.label.uppercase() else key.label
        Key.Language -> languageLabel
        is Key.Enter -> enterLabel.orEmpty()
        else -> key.label
    }

    /** 带一像素底影的圆角面。 */
    private fun drawPlate(canvas: Canvas, bounds: RectF, color: Int) {
        fill.color = palette.keyShadow
        canvas.drawRoundRect(
            bounds.left, bounds.top + metrics.shadowOffset,
            bounds.right, bounds.bottom + metrics.shadowOffset,
            metrics.radius, metrics.radius, fill,
        )
        fill.color = color
        canvas.drawRoundRect(bounds, metrics.radius, metrics.radius, fill)
    }

    private fun text(canvas: Canvas, value: String, sp: Float, color: Int, cx: Float, cy: Float) {
        ink.textSize = sp * density * scaleOf(sp)
        ink.color = color
        canvas.drawText(value, cx, cy - (ink.descent() + ink.ascent()) / 2, ink)
    }

    /** 同 [text]，放不下 [maxWidth] 就缩小字号（双拼韵母一个键可能有两三个）。 */
    private fun fitted(canvas: Canvas, value: String, sp: Float, color: Int, cx: Float, cy: Float, maxWidth: Float) {
        ink.textSize = sp * density * scaleOf(sp)
        val width = ink.measureText(value)
        text(canvas, value, if (width > maxWidth) sp * maxWidth / width else sp, color, cx, cy)
    }

    /** 带角标的键上主字的纵向位置（占键高的比例）：字放大时往下让一点，不顶到角标。 */
    private val belowHint = 0.6f + (textScale - 1f).coerceAtLeast(0f) * 0.15f

    /** 键面字号只放大键上的主字；角标、双拼韵母这类小字（[HINT_MAX_SP] 以下）不跟，放大了会和主字挤在一起。 */
    private fun scaleOf(sp: Float) = if (sp > HINT_MAX_SP) textScale else 1f

    private companion object {
        const val PRESSED_BRAND_SHADE = 0x33000000

        const val HINT_MAX_SP = 11f

        /** 锁定大写的小点：半径与离键右上角的距离。 */
        const val LOCK_DOT_DP = 3.5f
        const val LOCK_DOT_INSET_DP = 9f
    }
}
