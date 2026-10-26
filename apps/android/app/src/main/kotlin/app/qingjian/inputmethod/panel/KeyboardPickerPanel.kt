package app.qingjian.inputmethod.panel

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import app.qingjian.inputmethod.panel.base.PanelIcons.Icon
import app.qingjian.inputmethod.panel.base.PanelView
import app.qingjian.inputmethod.panel.base.Tabs
import app.qingjian.inputmethod.settings.Settings
import app.qingjian.inputmethod.theme.Palette

/**
 * 键盘选择：输入方式宫格，图标是「框里一个大字 + 右下角键数」，当前这个用品牌色加对勾徽章。
 * 选了就换键盘并回到键盘；英文 26 键只切到英文，拼音用哪种布局不变。
 */
@SuppressLint("ViewConstructor")
class KeyboardPickerPanel(
    context: Context,
    palette: Palette,
    actions: PanelActions,
    headerHeight: Float,
    private val current: Settings.Layout,
) : PanelView(context, palette, actions, headerHeight) {
    private enum class Mode { CHINESE, ENGLISH, JAPANESE }

    /** 一种输入方式；英文、日语不改中文布局，[layout] 为 null。 */
    private class Choice(val mode: Mode, val layout: Settings.Layout?, val glyph: String, val keys: String, val label: String)

    private val choices = listOf(
        Choice(Mode.CHINESE, Settings.Layout.NINE_KEY, "拼", "9", "拼音 9 键"),
        Choice(Mode.CHINESE, Settings.Layout.QWERTY, "拼", "26", "拼音 26 键"),
        Choice(Mode.CHINESE, Settings.Layout.SHUANGPIN, "双", "26", "双拼"),
        Choice(Mode.CHINESE, Settings.Layout.WUBI, "五", "86", "五笔 86 版"),
        Choice(Mode.CHINESE, Settings.Layout.ZHUYIN, "ㄅ", "", "注音 大千"),
        Choice(Mode.ENGLISH, null, "EN", "26", "英文 26 键"),
        Choice(Mode.JAPANESE, null, "あ", "26", "日语 罗马音"),
    )

    private val mode = when {
        actions.japanese -> Mode.JAPANESE
        actions.english -> Mode.ENGLISH
        else -> Mode.CHINESE
    }

    private fun chosen(choice: Choice) = choice.mode == mode && (mode != Mode.CHINESE || choice.layout == current)

    private val tabs = Tabs(ink, scrollable(true) { _, _ -> false }, Tabs.Style.UNDERLINE)
        .apply { labels = listOf("输入方式") }

    private val card = RectF()
    private val box = RectF()
    private val outline = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE }
    private val columns = 5

    override fun onDraw(canvas: Canvas) {
        drawHeader(canvas, tabs, Icon.SETTINGS)
        card.set(ink.dp(10f), headerHeight, width - ink.dp(10f), height - ink.dp(8f))
        ink.round(canvas, card, 16f, palette.surface)
        choices.forEachIndexed { i, choice ->
            val cx = card.left + cellWidth() * (i % columns + 0.5f)
            val cy = card.top + ink.dp(8f) + cellHeight() * (i / columns + 0.5f)
            val chosen = chosen(choice)
            val color = if (chosen) palette.brand else palette.text
            drawGlyph(canvas, choice, cx, cy - ink.dp(12f), color)
            if (chosen) drawBadge(canvas, cx + ink.dp(22f), cy - ink.dp(29f))
            ink.text(canvas, choice.label, 13f, if (chosen) palette.brand else palette.gloss, cx, cy + ink.dp(22f))
        }
    }

    private fun cellWidth() = card.width() / columns

    private fun cellHeight() = minOf(ink.dp(90f), card.height() / 2)

    private fun drawGlyph(canvas: Canvas, choice: Choice, cx: Float, cy: Float, color: Int) {
        box.set(cx - ink.dp(22f), cy - ink.dp(17f), cx + ink.dp(22f), cy + ink.dp(17f))
        outline.color = color
        outline.strokeWidth = ink.dp(1.7f)
        canvas.drawRoundRect(box, ink.dp(4f), ink.dp(4f), outline)
        ink.text(canvas, choice.glyph, if (choice.glyph.length > 1) 14f else 17f, color, cx - ink.dp(5f), cy - ink.dp(2f), bold = true)
        ink.text(canvas, choice.keys, 10f, color, box.right - ink.dp(10f), box.bottom - ink.dp(8f))
    }

    private fun drawBadge(canvas: Canvas, cx: Float, cy: Float) {
        ink.fill.color = palette.brand
        canvas.drawCircle(cx, cy, ink.dp(7.5f), ink.fill)
        ink.icon(canvas, Icon.CHECK, cx, cy, 13f, palette.onBrand)
    }

    override fun onTap(x: Float, y: Float) {
        if (isRightIcon(x, y)) {
            actions.openSettings()
            return
        }
        if (!card.contains(x, y)) return
        val row = ((y - card.top - ink.dp(8f)) / cellHeight()).toInt()
        val column = ((x - card.left) / cellWidth()).toInt()
        choices.getOrNull(row * columns + column)?.let { choice ->
            when (choice.mode) {
                Mode.CHINESE -> choice.layout?.let(actions::chooseLayout)
                Mode.ENGLISH -> actions.chooseEnglish()
                Mode.JAPANESE -> actions.chooseJapanese()
            }
        }
    }
}
