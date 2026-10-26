package app.qingjian.inputmethod.candidates

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import app.qingjian.inputmethod.core.frame.Candidate
import app.qingjian.inputmethod.core.frame.Sense
import app.qingjian.inputmethod.keyboard.layout.Key
import app.qingjian.inputmethod.keyboard.paint.KeyIcons
import app.qingjian.inputmethod.panel.PanelActions
import app.qingjian.inputmethod.panel.base.PanelIcons.Icon
import app.qingjian.inputmethod.panel.base.PanelView
import app.qingjian.inputmethod.theme.Palette

/**
 * 候选展开：候选条右端点 ⌄ 打开，盖住键盘。顶上一条是拼音与收起 ⌃，下面候选按字宽往一行里排、
 * 排不下换行，每行左右撑满，上下滑；右列 返回 / 删除 / 重输。词下面照候选条带第一条译文。
 */
@SuppressLint("ViewConstructor")
class CandidateGrid(
    context: Context,
    palette: Palette,
    actions: PanelActions,
    headerHeight: Float,
    /** 候选字号档的倍数（[app.qingjian.inputmethod.theme.TextSize]），词、译文、云朵与行高一起缩放。 */
    private val textScale: Float,
) : PanelView(context, palette, actions, headerHeight) {
    override val hasHeader = false

    private var preedit = ""
    private var candidates: List<Candidate> = emptyList()

    /** 排好的格子（内容坐标，未减滚动），下标同 [candidates]。 */
    private val cells = mutableListOf<RectF>()
    private var laidOutWidth = 0

    private val body = RectF()
    private val side = RectF()
    private val scroll = scrollable(horizontal = false) { x, y -> body.contains(x, y) }
    private val keyPaint = Paint(Paint.ANTI_ALIAS_FLAG)
    private val wordSp = WORD_SP * textScale
    private val glossSp = GLOSS_SP * textScale
    private val cloudDp = CLOUD_DP * textScale

    fun show(preedit: String, candidates: List<Candidate>) {
        this.preedit = preedit
        this.candidates = candidates
        laidOutWidth = 0
        scroll.reset()
        invalidate()
    }

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
        body.set(0f, headerHeight, w - ink.dp(SIDE_DP), h.toFloat())
        side.set(body.right, headerHeight, w.toFloat(), h.toFloat())
        laidOutWidth = 0
    }

    /** 按字宽往一行里装，装不下换行；一行装完把剩下的宽度平分给这一行的格子。 */
    private fun layout() {
        if (laidOutWidth == width) return
        laidOutWidth = width
        cells.clear()
        val rowWidth = body.width()
        val minWidth = rowWidth / MAX_PER_ROW
        val rowHeight = ink.dp(ROW_DP * textScale)
        var rowStart = 0
        var used = 0f
        var top = 0f
        fun finishRow() {
            val row = cells.subList(rowStart, cells.size)
            if (row.isEmpty()) return
            val extra = (rowWidth - used) / row.size
            var x = 0f
            for (cell in row) {
                val w = cell.width() + extra
                cell.set(x, top, x + w, top + rowHeight)
                x += w
            }
            rowStart = cells.size
            used = 0f
            top += rowHeight
        }
        for (candidate in candidates) {
            val gloss = candidate.senses.firstOrNull()?.let(::glossText).orEmpty()
            val word = ink.measure(candidate.text, wordSp) + if (candidate.cloud) ink.dp(cloudDp + CLOUD_GAP_DP) else 0f
            val natural = maxOf(word, ink.measure(gloss, glossSp)) + ink.dp(PAD_DP * 2)
            val w = natural.coerceIn(minWidth, rowWidth)
            if (used + w > rowWidth) finishRow()
            cells += RectF(0f, 0f, w, rowHeight)
            used += w
        }
        finishRow()
        scroll.max = top - body.height()
    }

    private fun glossText(sense: Sense) =
        listOfNotNull(sense.pos, sense.text).joinToString(" ")

    override fun onDraw(canvas: Canvas) {
        layout()
        drawHeader(canvas)
        drawCells(canvas)
        drawSide(canvas)
    }

    private fun drawHeader(canvas: Canvas) {
        ink.fill.color = palette.surface
        canvas.drawRect(0f, 0f, width.toFloat(), headerHeight, ink.fill)
        val collapse = ink.dp(COLLAPSE_DP)
        val cy = headerHeight / 2
        ink.textStart(canvas, preedit, HEADER_SP, palette.text, ink.dp(PAD_DP + 2), cy, width - collapse - ink.dp(PAD_DP))
        ink.icon(canvas, Icon.COLLAPSE, width - collapse / 2, cy, 26f, palette.text)
        ink.line(canvas, 0f, headerHeight, width.toFloat(), headerHeight, palette.faint)
    }

    private fun drawCells(canvas: Canvas) {
        canvas.save()
        canvas.clipRect(body)
        canvas.translate(body.left, body.top - scroll.offset)
        val visibleTop = scroll.offset
        val visibleBottom = scroll.offset + body.height()
        for ((index, cell) in cells.withIndex()) {
            if (cell.bottom < visibleTop || cell.top > visibleBottom) continue
            val candidate = candidates[index]
            val gloss = candidate.senses.firstOrNull()
            val maxWidth = cell.width() - ink.dp(PAD_DP)
            val wordColor = when {
                index == 0 -> palette.brand
                candidate.cloud -> palette.cloud
                else -> palette.text
            }
            val wordY = if (gloss == null) cell.centerY() else cell.centerY() - ink.dp(8f * textScale)
            // 云端候选：词前带小云朵，词与云朵一起居中
            val cloudWidth = if (candidate.cloud) ink.dp(cloudDp + CLOUD_GAP_DP) else 0f
            val wordX = cell.centerX() + cloudWidth / 2
            ink.text(canvas, candidate.text, wordSp, wordColor, wordX, wordY, maxWidth = maxWidth - cloudWidth)
            if (candidate.cloud) {
                val shown = minOf(ink.measure(candidate.text, wordSp), maxWidth - cloudWidth)
                ink.icon(canvas, Icon.CLOUD, wordX - shown / 2 - cloudWidth + ink.dp(cloudDp) / 2, wordY, cloudDp, palette.cloud)
            }
            if (gloss != null) {
                ink.text(
                    canvas, glossText(gloss), glossSp, if (gloss.fresh) palette.fresh else palette.gloss,
                    cell.centerX(), cell.centerY() + ink.dp(13f * textScale), maxWidth = maxWidth,
                )
            }
            if (cell.right < body.width() - 1) {
                ink.line(canvas, cell.right, cell.top + ink.dp(14f), cell.right, cell.bottom - ink.dp(14f), palette.faint)
            }
            if (cell.left == 0f) ink.line(canvas, 0f, cell.bottom, body.width(), cell.bottom, palette.faint)
        }
        canvas.restore()
    }

    private fun drawSide(canvas: Canvas) {
        ink.fill.color = palette.functionalKey
        canvas.drawRect(side, ink.fill)
        val cx = side.centerX()
        val slot = side.height() / 3
        ink.text(canvas, "返回", 16f, palette.text, cx, side.top + slot * 0.5f)
        keyPaint.color = palette.text
        KeyIcons.draw(canvas, Key.Backspace, cx, side.top + slot * 1.5f, ink.dp(24f), keyPaint, false)
        ink.text(canvas, "重输", 16f, palette.text, cx, side.top + slot * 2.5f)
        for (i in 1..2) ink.line(canvas, side.left, side.top + slot * i, side.right, side.top + slot * i, palette.faint)
    }

    override fun onTap(x: Float, y: Float) {
        when {
            y < headerHeight -> if (x > width - ink.dp(COLLAPSE_DP)) actions.closePanel()
            side.contains(x, y) -> when (((y - side.top) / (side.height() / 3)).toInt()) {
                0 -> actions.closePanel()
                1 -> actions.deleteBackward()
                else -> actions.clearComposition()
            }
            body.contains(x, y) -> {
                val cx = x - body.left
                val cy = y - body.top + scroll.offset
                val index = cells.indexOfFirst { it.contains(cx, cy) }
                if (index >= 0) actions.selectCandidate(index)
            }
        }
    }

    private companion object {
        const val SIDE_DP = 64f
        const val COLLAPSE_DP = 56f
        const val ROW_DP = 58f
        const val PAD_DP = 14f
        const val WORD_SP = 19f
        const val CLOUD_DP = 16f
        const val CLOUD_GAP_DP = 4f
        const val GLOSS_SP = 11f
        const val HEADER_SP = 16f

        /** 一行最多几个：最窄的格子是一行的这么几分之一，单字候选也不会挤成一长排。 */
        const val MAX_PER_ROW = 5
    }
}
