package app.qingjian.inputmethod.keyboard.paint

import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import app.qingjian.inputmethod.keyboard.SideColumn
import app.qingjian.inputmethod.keyboard.layout.Key
import app.qingjian.inputmethod.keyboard.layout.KeySlot
import app.qingjian.inputmethod.keyboard.layout.KeyboardMetrics
import app.qingjian.inputmethod.theme.Palette

/** 画键与按键气泡。 */
class KeyPainter(private val palette: Palette, private val metrics: KeyboardMetrics) {
    private val density = metrics.density
    private val fill = Paint(Paint.ANTI_ALIAS_FLAG)
    private val ink = Paint(Paint.ANTI_ALIAS_FLAG).apply { textAlign = Paint.Align.CENTER }
    private val bubble = RectF()

    var shifted = false
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
                down || (key == Key.Shift && shifted) -> palette.accent
                key.functional -> palette.functionalKey
                else -> palette.surface
            },
        )
        if (brand && down) {
            fill.color = PRESSED_BRAND_SHADE
            canvas.drawRoundRect(bounds, metrics.radius, metrics.radius, fill)
        }

        // 角标先画会把 ink 换成浅色，正文的颜色另存一份
        val labelColor = if (brand) palette.onBrand else palette.text
        ink.color = labelColor
        val label = label(key)
        when {
            label.isEmpty() ->
                KeyIcons.draw(canvas, key, bounds.centerX(), bounds.centerY(), 24 * density, ink, shifted)

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
                text(canvas, label, 21f, labelColor, bounds.centerX(), bounds.top + bounds.height() * 0.6f)
            }

            // 中英键：角上写按一下会切到哪种语言，长按换
            key == Key.Language && languageHint.isNotEmpty() -> {
                text(canvas, languageHint, 10f, palette.gloss, bounds.right - bounds.width() * 0.22f, bounds.top + bounds.height() * 0.24f)
                text(canvas, label, 17f, labelColor, bounds.centerX(), bounds.centerY())
            }

            key is Key.Digit -> {
                text(canvas, key.digit.toString(), 11f, palette.faint, bounds.centerX(), bounds.top + bounds.height() * 0.24f)
                text(canvas, label, 18f, palette.text, bounds.centerX(), bounds.top + bounds.height() * 0.6f)
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

    /** 按下的键上方放大显示；顶排的气泡会画到候选条上。 */
    fun drawBubble(canvas: Canvas, slot: KeySlot, viewWidth: Int) {
        val bounds = slot.bounds
        val grow = 7f * density
        bubble.set(
            (bounds.left - grow).coerceAtLeast(metrics.gapX),
            bounds.top - metrics.rowHeight - 2 * density,
            (bounds.right + grow).coerceAtMost(viewWidth - metrics.gapX),
            bounds.top - 4 * density,
        )
        drawPlate(canvas, bubble, palette.surface)
        text(canvas, label(slot.key), 30f, palette.text, bubble.centerX(), bubble.centerY())
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
        is Key.Letter -> if (shifted || chinese) key.label.uppercase() else key.label
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
        ink.textSize = sp * density
        ink.color = color
        canvas.drawText(value, cx, cy - (ink.descent() + ink.ascent()) / 2, ink)
    }

    /** 同 [text]，放不下 [maxWidth] 就缩小字号（双拼韵母一个键可能有两三个）。 */
    private fun fitted(canvas: Canvas, value: String, sp: Float, color: Int, cx: Float, cy: Float, maxWidth: Float) {
        ink.textSize = sp * density
        val width = ink.measureText(value)
        text(canvas, value, if (width > maxWidth) sp * maxWidth / width else sp, color, cx, cy)
    }

    private companion object {
        const val PRESSED_BRAND_SHADE = 0x33000000
    }
}
