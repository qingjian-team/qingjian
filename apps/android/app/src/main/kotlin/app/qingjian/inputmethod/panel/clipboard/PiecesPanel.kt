package app.qingjian.inputmethod.panel.clipboard

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.RectF
import android.view.MotionEvent
import app.qingjian.inputmethod.keyboard.Haptics
import app.qingjian.inputmethod.panel.PanelActions
import app.qingjian.inputmethod.panel.PanelKind
import app.qingjian.inputmethod.panel.base.PanelView
import app.qingjian.inputmethod.panel.base.Tabs
import app.qingjian.inputmethod.theme.Palette
import kotlin.math.abs

/**
 * 剪贴板的「拆词」：一段文字按 Core 的 `text_pieces` 切成小块流式铺开，点一块选中 / 取消，
 * 空白也是块（空格画 ␣、换行画 ↵，面板里不真的换行，块连着排），选中的按原文顺序原样拼起来，代码的换行缩进都在；
 * 拼好的实时放进输入框、处在编辑中（组合文字），选的和输入框里的始终一样；
 * 横着滑（或长按后拖）连选：起点到手指之间整段跟着起点那块反过来；竖着滑是滚动。
 * 底下 全选 / 复制 / 完成。点返回（回剪贴板）或完成（回键盘）都确认上屏。
 */
@SuppressLint("ViewConstructor")
class PiecesPanel(
    context: Context,
    palette: Palette,
    actions: PanelActions,
    headerHeight: Float,
    text: String,
) : PanelView(context, palette, actions, headerHeight) {
    private val pieces = actions.textPieces(text)
    private val selected = BooleanArray(pieces.size)
    private val chipLabels = pieces.map(::labelOf)

    private val tabs = Tabs(ink, scrollable(true) { _, _ -> false }, Tabs.Style.UNDERLINE).apply {
        labels = listOf("滑动选词")
    }

    private val area = RectF()
    private val areaScroll = scrollable(false) { x, y -> area.contains(x, y) }
    private val chip = RectF()
    private val button = RectF()
    private var lefts = FloatArray(0)
    private var tops = FloatArray(0)
    private var widths = FloatArray(0)

    private val buttons = listOf("全选", "复制", "完成")

    private fun barTop() = height - ink.dp(BAR_DP)

    override fun onDraw(canvas: Canvas) {
        drawHeader(canvas, tabs)
        area.set(0f, headerHeight + ink.dp(4f), width.toFloat(), barTop())
        arrange()
        canvas.save()
        canvas.clipRect(area)
        val rowHeight = ink.dp(CHIP_DP)
        for (i in pieces.indices) {
            val top = area.top + tops[i] - areaScroll.offset
            if (top + rowHeight < area.top || top > area.bottom) continue
            chip.set(lefts[i], top, lefts[i] + widths[i], top + rowHeight)
            ink.round(canvas, chip, 8f, if (selected[i]) palette.accent else palette.surface)
            val blank = pieces[i].isBlank()
            ink.text(canvas, chipLabels[i], if (blank) 14f else 16f, if (blank) palette.gloss else palette.text, chip.centerX(), chip.centerY(), bold = selected[i])
        }
        canvas.restore()
        val each = (width - ink.dp(20f)) / buttons.size
        buttons.forEachIndexed { i, label ->
            button.set(ink.dp(10f) + each * i + ink.dp(4f), barTop() + ink.dp(8f), ink.dp(10f) + each * (i + 1) - ink.dp(4f), height - ink.dp(8f))
            val primary = i == buttons.lastIndex
            ink.round(canvas, button, 12f, if (primary) palette.brand else palette.surface)
            ink.text(canvas, label, 15f, if (primary) palette.onBrand else palette.text, button.centerX(), button.centerY(), bold = primary)
        }
    }

    /** 流式排：一行排不下换行，块宽按字量。 */
    private fun arrange() {
        lefts = FloatArray(pieces.size)
        tops = FloatArray(pieces.size)
        widths = FloatArray(pieces.size)
        val gap = ink.dp(6f)
        val right = width - ink.dp(10f)
        var x = ink.dp(10f)
        var y = ink.dp(4f)
        for (i in pieces.indices) {
            val padding = ink.dp(if (pieces[i].isBlank()) 12f else 24f)
            val w = (ink.measure(chipLabels[i], 16f) + padding).coerceAtMost(right - ink.dp(10f))
            if (x + w > right && x > ink.dp(10f)) {
                x = ink.dp(10f)
                y += ink.dp(CHIP_DP) + gap
            }
            lefts[i] = x
            tops[i] = y
            widths[i] = w
            x += w + gap
        }
        areaScroll.max = y + ink.dp(CHIP_DP) + gap - area.height()
    }

    /** 滑动连选：起点那块与手指之间按原文顺序整段设成 [sweepValue]；-1 是没在滑。 */
    private var sweepStart = -1
    private var sweepEnd = -1
    private var sweepValue = true
    private var sweepBase = BooleanArray(0)
    private var downX = 0f
    private var downY = 0f
    private var downIndex = -1

    private fun chipAt(x: Float, y: Float): Int? {
        if (!area.contains(x, y)) return null
        val contentY = y - area.top + areaScroll.offset
        return pieces.indices.firstOrNull {
            x >= lefts[it] && x < lefts[it] + widths[it] && contentY >= tops[it] && contentY < tops[it] + ink.dp(CHIP_DP)
        }
    }

    /** 手指不一定正压在块上（块之间有缝）：取这一行里离它最近的块，行外的取最近一行。 */
    private fun nearestChip(x: Float, y: Float): Int? {
        if (pieces.isEmpty()) return null
        val contentY = y.coerceIn(area.top, area.bottom) - area.top + areaScroll.offset
        val row = pieces.indices.minByOrNull { abs(tops[it] + ink.dp(CHIP_DP) / 2 - contentY) } ?: return null
        return pieces.indices.filter { tops[it] == tops[row] }.minByOrNull {
            abs(lefts[it] + widths[it] / 2 - x)
        }
    }

    private fun startSweep(i: Int) {
        sweepStart = i
        sweepEnd = -1
        sweepValue = !selected[i]
        sweepBase = selected.copyOf()
        sweepTo(i)
    }

    private fun sweepTo(i: Int) {
        if (i == sweepEnd) return
        sweepEnd = i
        sweepBase.copyInto(selected)
        for (k in minOf(sweepStart, i)..maxOf(sweepStart, i)) selected[k] = sweepValue
        Haptics.tap(context)
        actions.composeText(chosen())
        invalidate()
    }

    override fun onHold(x: Float, y: Float, rawX: Float) {
        chipAt(x, y)?.let(::startSweep)
    }

    @SuppressLint("ClickableViewAccessibility")
    override fun onTouchEvent(event: MotionEvent): Boolean {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                sweepStart = -1
                downX = event.x
                downY = event.y
                downIndex = chipAt(event.x, event.y) ?: -1
            }
            MotionEvent.ACTION_MOVE -> if (sweepStart < 0 && downIndex >= 0) {
                val dx = abs(event.x - downX)
                val dy = abs(event.y - downY)
                // 先横着走才算连选，竖着走仍是滚动
                if (dx > ink.dp(SLOP_DP) && dx > dy) {
                    startSweep(downIndex)
                    cancelGestures(event)
                }
            }
        }
        if (sweepStart < 0) return super.onTouchEvent(event)
        when (event.actionMasked) {
            MotionEvent.ACTION_MOVE -> nearestChip(event.x, event.y)?.let(::sweepTo)
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                sweepStart = -1
                cancelGestures(event)
            }
        }
        return true
    }

    /** 进了连选就让手势识别作废这一下，免得抬手又算点击或长按。 */
    private fun cancelGestures(event: MotionEvent) {
        val cancel = MotionEvent.obtain(event).apply { action = MotionEvent.ACTION_CANCEL }
        super.onTouchEvent(cancel)
        cancel.recycle()
    }

    override fun onBack() {
        actions.finishComposeText()
        actions.openPanel(PanelKind.CLIPBOARD)
    }

    override fun onTap(x: Float, y: Float) {
        if (y > barTop()) {
            val index = ((x - ink.dp(10f)) / ((width - ink.dp(20f)) / buttons.size)).toInt()
            when (index) {
                0 -> {
                    selected.fill(!selected.all { it })
                    actions.composeText(chosen())
                }
                1 -> chosen().takeIf { it.isNotEmpty() }?.let(actions::copyToClipboard)
                2 -> {
                    actions.finishComposeText()
                    actions.closePanel()
                }
            }
            return
        }
        val i = chipAt(x, y) ?: return
        selected[i] = !selected[i]
        actions.composeText(chosen())
    }

    /** 选中的块按原文顺序原样拼起来（空白、换行也是块，全选就是原文）。 */
    private fun chosen(): String = pieces.filterIndexed { i, _ -> selected[i] }.joinToString("")

    private fun String.isNewline() = this == "\n" || this == "\r\n" || this == "\r"

    /** 块上写的字：换行画 ↵，空格画 ␣（几个就带个数），制表符画 ⇥。 */
    private fun labelOf(piece: String): String = when {
        piece.isNewline() -> "↵"
        piece.isBlank() -> piece.groupBy { it }.entries.joinToString(" ") { (c, run) ->
            val mark = if (c == '\t') "⇥" else "␣"
            if (run.size > 1) "$mark${run.size}" else mark
        }
        else -> piece
    }

    private companion object {
        const val CHIP_DP = 40f
        const val BAR_DP = 60f
        const val SLOP_DP = 10f
    }
}
