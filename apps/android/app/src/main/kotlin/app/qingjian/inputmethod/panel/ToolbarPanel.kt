package app.qingjian.inputmethod.panel

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.RectF
import app.qingjian.inputmethod.panel.base.PanelIcons.Icon
import app.qingjian.inputmethod.panel.base.PanelView
import app.qingjian.inputmethod.panel.base.Tabs
import app.qingjian.inputmethod.settings.Settings
import app.qingjian.inputmethod.settings.ToolbarTool
import app.qingjian.inputmethod.theme.Palette

/**
 * 定制工具栏（#777，功能面板「常用功能」里进），在键盘区原地编辑：上面一排是工具栏上的、按顺序，右上角「−」点了移除；
 * 下面是全部工具，放了的高亮，点没放的加到末尾、点放了的移除。左上角 ✓ 完成才写进设置并回到键盘，
 * 系统返回键放弃这次改动；右上角「恢复默认」先换成 [ToolbarTool.DEFAULT]，同样要点 ✓ 才算。
 */
@SuppressLint("ViewConstructor")
class ToolbarPanel(
    context: Context,
    palette: Palette,
    actions: PanelActions,
    headerHeight: Float,
    private val saved: List<ToolbarTool>,
) : PanelView(context, palette, actions, headerHeight) {
    private val tabs = Tabs(ink, scrollable(true) { _, _ -> false }, Tabs.Style.UNDERLINE)
        .apply { labels = listOf("定制工具栏") }

    private val card = RectF()
    private val scroll = scrollable(false) { x, y -> card.contains(x, y) }
    private val columns = 5

    /** 正在编辑的工具栏，点 ✓ 才存。 */
    private var placed = saved

    private val sectionTitle get() = ink.dp(34f)
    private val cellHeight get() = ink.dp(68f)

    private fun rows(count: Int) = maxOf(1, (count + columns - 1) / columns)

    /** 下面「全部工具」那一节在内容里的起点（相对卡片顶，未滚动）。 */
    private val allTop get() = sectionTitle + rows(placed.size) * cellHeight

    private val contentHeight get() = allTop + sectionTitle + rows(ToolbarTool.entries.size) * cellHeight + ink.dp(8f)

    private fun cellWidth() = card.width() / columns

    private enum class Badge { REMOVE, ADD, PLACED }

    override fun onDraw(canvas: Canvas) {
        drawHeader(canvas, tabs, leftIcon = Icon.CHECK)
        ink.text(canvas, RESET, 14f, palette.brand, width - ink.dp(RESET_WIDTH_DP) / 2, headerHeight / 2)
        card.set(ink.dp(10f), headerHeight, width - ink.dp(10f), height - ink.dp(8f))
        ink.round(canvas, card, 16f, palette.surface)
        scroll.max = contentHeight - card.height()
        canvas.save()
        canvas.clipRect(card)
        val top = card.top - scroll.offset
        drawTitle(canvas, "工具栏（按这个顺序排）", top)
        if (placed.isEmpty()) {
            ink.text(canvas, "中间空着，从下面加", 13f, palette.gloss, card.centerX(), top + sectionTitle + cellHeight / 2)
        }
        placed.forEachIndexed { i, tool -> drawCell(canvas, tool, i, top + sectionTitle, Badge.REMOVE) }
        drawTitle(canvas, "全部工具（点一下加到末尾）", top + allTop)
        ToolbarTool.entries.forEachIndexed { i, tool ->
            drawCell(canvas, tool, i, top + allTop + sectionTitle, if (tool in placed) Badge.PLACED else Badge.ADD)
        }
        canvas.restore()
    }

    private fun drawTitle(canvas: Canvas, title: String, top: Float) {
        ink.textStart(canvas, title, 12f, palette.gloss, card.left + ink.dp(16f), top + sectionTitle / 2 + ink.dp(4f), card.right - ink.dp(16f))
    }

    private fun drawCell(canvas: Canvas, tool: ToolbarTool, index: Int, top: Float, badge: Badge) {
        val cx = card.left + cellWidth() * (index % columns + 0.5f)
        val cy = top + cellHeight * (index / columns + 0.5f)
        val highlighted = badge == Badge.PLACED
        ink.icon(canvas, tool.icon, cx, cy - ink.dp(10f), 26f, if (highlighted) palette.brand else palette.text)
        ink.text(canvas, tool.label, 12f, if (highlighted) palette.brand else palette.gloss, cx, cy + ink.dp(18f), maxWidth = cellWidth() - ink.dp(4f))
        val bx = cx + ink.dp(17f)
        val by = cy - ink.dp(24f)
        ink.fill.color = if (highlighted) palette.brand else palette.functionalKey
        canvas.drawCircle(bx, by, ink.dp(7.5f), ink.fill)
        val mark = when (badge) {
            Badge.REMOVE -> Icon.MINUS
            Badge.ADD -> Icon.PLUS
            Badge.PLACED -> Icon.CHECK
        }
        ink.icon(canvas, mark, bx, by, 13f, if (highlighted) palette.onBrand else palette.text)
    }

    /** 左上角是 ✓：存下并回到键盘。 */
    override fun onBack() {
        actions.closePanel()
        if (placed != saved) actions.setChoice(Settings.KEY_TOOLBAR, ToolbarTool.format(placed))
    }

    override fun onTap(x: Float, y: Float) {
        if (y < headerHeight) {
            if (x > width - ink.dp(RESET_WIDTH_DP)) placed = ToolbarTool.DEFAULT
            return
        }
        if (!card.contains(x, y)) return
        val inContent = y - card.top + scroll.offset
        val column = ((x - card.left) / cellWidth()).toInt().coerceIn(0, columns - 1)
        if (inContent >= allTop + sectionTitle) {
            val row = ((inContent - allTop - sectionTitle) / cellHeight).toInt()
            val tool = ToolbarTool.entries.getOrNull(row * columns + column) ?: return
            placed = if (tool in placed) placed - tool else placed + tool
        } else if (inContent in sectionTitle..allTop) {
            val row = ((inContent - sectionTitle) / cellHeight).toInt()
            placed.getOrNull(row * columns + column)?.let { placed = placed - it }
        }
    }

    private companion object {
        const val RESET = "恢复默认"
        const val RESET_WIDTH_DP = 100f
    }
}
