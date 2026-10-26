package app.qingjian.inputmethod.candidates

import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.panel.base.PanelIcons
import app.qingjian.inputmethod.panel.base.PanelIcons.Icon
import app.qingjian.inputmethod.theme.Palette

/**
 * 不在组句时候选条的样子：左边 logo 进功能面板、右边收起键盘，两头固定、各隔一道竖线，宽度一样；
 * 中间一排图标一屏正好放 [VISIBLE_SLOTS] 个，多出来的（翻译、调高度）横着滑出来。
 */
class Toolbar(context: Context, private val palette: Palette, private val density: Float, private val barHeight: Float) {
    enum class Action { FUNCTIONS, EMOJI, KEYBOARDS, VOICE, SYMBOLS, CLIPBOARD, TRANSLATE, RESIZE, HIDE_KEYBOARD }

    private val logo = context.getDrawable(R.drawable.ic_brand)
    private val logoSlot = 62f * density
    private val hideSlot = 62f * density
    private val middle = listOf(
        Action.EMOJI to Icon.EMOJI,
        Action.KEYBOARDS to Icon.KEYBOARD,
        Action.VOICE to Icon.MIC,
        Action.SYMBOLS to Icon.SYMBOLS,
        Action.CLIPBOARD to Icon.CLIPBOARD,
        Action.TRANSLATE to Icon.TRANSLATE,
        Action.RESIZE to Icon.RESIZE,
    )
    private val stroke = Paint(Paint.ANTI_ALIAS_FLAG)
    private val line = Paint().apply { color = palette.faint }
    private val card = Paint(Paint.ANTI_ALIAS_FLAG)
    private val cardRect = RectF()

    /** 中间那排滑过去的距离（像素）。 */
    var offset = 0f
        private set

    private fun middleSlot(viewWidth: Int) = (viewWidth - logoSlot - hideSlot) / VISIBLE_SLOTS

    fun maxOffset(viewWidth: Int) = (middleSlot(viewWidth) * middle.size - (viewWidth - logoSlot - hideSlot)).coerceAtLeast(0f)

    fun scrollTo(value: Float, viewWidth: Int) {
        offset = value.coerceIn(0f, maxOffset(viewWidth))
    }

    /** [x] 落在中间那排上（可滑的区域）。 */
    fun inMiddle(x: Float, viewWidth: Int) = x >= logoSlot && x <= viewWidth - hideSlot

    fun actionAt(x: Float, viewWidth: Int): Action = when {
        x < logoSlot -> Action.FUNCTIONS
        x > viewWidth - hideSlot -> Action.HIDE_KEYBOARD
        else -> middle[((x - logoSlot + offset) / middleSlot(viewWidth)).toInt().coerceIn(0, middle.size - 1)].first
    }

    /** 浮在键盘上方的一块圆角卡片，底下一道与按键同样的底影，和键盘分出层次。 */
    private fun drawCard(canvas: Canvas, viewWidth: Int) {
        val radius = 14 * density
        cardRect.set(6 * density, 6 * density, viewWidth - 6 * density, barHeight - 4 * density)
        card.color = palette.keyShadow
        cardRect.offset(0f, density)
        canvas.drawRoundRect(cardRect, radius, radius, card)
        cardRect.offset(0f, -density)
        card.color = palette.surface
        canvas.drawRoundRect(cardRect, radius, radius, card)
    }

    fun draw(canvas: Canvas, viewWidth: Int) {
        val cy = barHeight / 2
        drawCard(canvas, viewWidth)
        // 与 Windows 托盘同一张品牌标
        val logoSize = (30 * density).toInt()
        val logoCentre = (logoSlot / 2).toInt()
        logo?.setBounds(
            logoCentre - logoSize / 2, (cy - logoSize / 2).toInt(),
            logoCentre + logoSize / 2, (cy + logoSize / 2).toInt(),
        )
        logo?.draw(canvas)

        stroke.color = palette.gloss
        val slot = middleSlot(viewWidth)
        val divider = viewWidth - hideSlot
        canvas.save()
        canvas.clipRect(logoSlot, 0f, divider, barHeight)
        middle.forEachIndexed { i, (_, icon) ->
            PanelIcons.draw(canvas, icon, logoSlot + slot * (i + 0.5f) - offset, cy, 26 * density, stroke)
        }
        canvas.restore()
        line.strokeWidth = density
        canvas.drawLine(logoSlot, cy - 12 * density, logoSlot, cy + 12 * density, line)
        canvas.drawLine(divider, cy - 12 * density, divider, cy + 12 * density, line)
        PanelIcons.draw(canvas, Icon.HIDE, viewWidth - hideSlot / 2, cy, 24 * density, stroke)
    }

    private companion object {
        const val VISIBLE_SLOTS = 5f
    }
}
