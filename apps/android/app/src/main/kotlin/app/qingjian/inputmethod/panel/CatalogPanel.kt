package app.qingjian.inputmethod.panel

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import android.view.MotionEvent
import app.qingjian.inputmethod.core.store.Catalog
import app.qingjian.inputmethod.core.store.CatalogSection
import app.qingjian.inputmethod.keyboard.Haptics
import app.qingjian.inputmethod.keyboard.layout.Key
import app.qingjian.inputmethod.keyboard.paint.KeyIcons
import app.qingjian.inputmethod.panel.base.PanelIcons.Icon
import app.qingjian.inputmethod.panel.base.PanelView
import app.qingjian.inputmethod.panel.base.Tabs
import app.qingjian.inputmethod.popup.ChoicePopup
import app.qingjian.inputmethod.theme.Palette
import kotlin.math.abs
import kotlin.math.hypot

/**
 * 符号与表情面板，内容是 Core 的面板目录（[Catalog]），点一格上屏。
 * - 符号：不要表头，细线分格；右边一列 返回 / 删除 / 锁定，没锁上屏一个就回键盘；底下横排分类。
 * - 表情：表头是 Emoji / 颜文字 两大类，底下横排小类，右下角删除；上屏后留在面板里接着选。
 */
@SuppressLint("ViewConstructor")
class CatalogPanel(
    context: Context,
    palette: Palette,
    actions: PanelActions,
    headerHeight: Float,
    kind: PanelKind,
) : PanelView(context, palette, actions, headerHeight) {
    private val symbols = kind == PanelKind.SYMBOLS

    override val hasHeader get() = !symbols

    private val groupName = if (symbols) Catalog.SYMBOLS else Catalog.EMOJI

    // 「常用」按用户用过的排，每次打开面板取一次；开着时不重排，格子不会在手指底下挪
    private val all: List<CatalogSection> = Catalog.group(groupName).map { section ->
        if (section.name != Catalog.COMMON) return@map section
        CatalogSection(section.group, section.name, Catalog.common(context, groupName) ?: section.items)
    }
    private val emoji = all.filter { it.name != Catalog.KAOMOJI }
    private val kaomoji = all.filter { it.name == Catalog.KAOMOJI }

    private val grid = RectF()
    private val bottom = RectF()
    private val side = RectF()
    private val keyPaint = Paint(Paint.ANTI_ALIAS_FLAG)

    private val popup = ChoicePopup(this, palette, titleSp = 26f)
    private var choices: List<String> = emptyList()
    private var pressed: String? = null
    private val pressedCell = RectF()
    private val scratch = RectF()
    private val bubble = RectF()
    private var downX = 0f
    private var downY = 0f
    private var holdRawX = 0f
    private var tracking = false
    private val location = IntArray(2)

    private val gridScroll = scrollable(false) { x, y -> grid.contains(x, y) }

    private val top = Tabs(ink, scrollable(true) { _, _ -> false }, Tabs.Style.UNDERLINE).apply {
        labels = listOf("Emoji", "颜文字")
        selected = emojiTop
    }

    private val categories = Tabs(ink, scrollable(true) { x, y -> bottomTabsContain(x, y) }, Tabs.Style.PILL).apply {
        labels = sections().map { it.name }
        selected = (if (symbols) symbolSection else emojiSection).coerceIn(0, (labels.size - 1).coerceAtLeast(0))
    }

    /** 当前大类下的分类：符号就是全部；表情看顶上选的是 Emoji 还是颜文字。 */
    private fun sections() = when {
        symbols -> all
        top.selected == 1 -> kaomoji
        else -> emoji
    }

    private val section get() = sections().getOrNull(categories.selected)

    private fun bottomTabsContain(x: Float, y: Float) = y > bottom.top && x < bottom.right - ink.dp(BOTTOM_KEY_DP)

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
        val bottomHeight = ink.dp(BOTTOM_DP)
        bottom.set(0f, h - bottomHeight, w.toFloat(), h.toFloat())
        if (symbols) {
            side.set(w - w / SIDE_FRACTION, 0f, w.toFloat(), h - bottomHeight)
            grid.set(0f, 0f, side.left, h - bottomHeight)
        } else {
            grid.set(0f, headerHeight, w.toFloat(), h - bottomHeight)
        }
        gridScroll.reset()
    }

    /** 一行几格：emoji 八格；符号一字一格五格，宽的（颜文字、单位）三格或两格。 */
    private fun columns(): Int {
        val section = section ?: return 1
        if (!symbols) return if (section.name == Catalog.KAOMOJI) 3 else 8
        val widest = section.items.maxOfOrNull { item -> item.sumOf { if (it.code < 0x80) 6 else 10 } } ?: 10
        return when {
            widest <= 20 -> 5
            widest <= 50 -> 3
            else -> 2
        }
    }

    private fun rowHeight(columns: Int) = when {
        symbols -> grid.height() / 4
        columns == 8 -> grid.width() / 8
        else -> ink.dp(52f)
    }

    override fun onDraw(canvas: Canvas) {
        if (!symbols) drawHeader(canvas, top)
        drawGrid(canvas)
        if (symbols) drawSide(canvas)
        drawBottom(canvas)
        drawPressed(canvas)
    }

    private fun drawGrid(canvas: Canvas) {
        val items = section?.items.orEmpty()
        val columns = columns()
        val cellWidth = grid.width() / columns
        val rowHeight = rowHeight(columns)
        val rows = (items.size + columns - 1) / columns
        gridScroll.max = rows * rowHeight - grid.height()
        val textSize = when {
            !symbols && columns == 8 -> 27f
            columns >= 5 -> 22f
            else -> 16f
        }
        canvas.save()
        canvas.clipRect(grid)
        val first = (gridScroll.offset / rowHeight).toInt()
        val last = ((gridScroll.offset + grid.height()) / rowHeight).toInt()
        for (row in first..minOf(last, rows - 1)) {
            val y = grid.top + row * rowHeight - gridScroll.offset
            for (column in 0 until columns) {
                val item = items.getOrNull(row * columns + column) ?: break
                val cx = grid.left + cellWidth * (column + 0.5f)
                ink.text(canvas, item, textSize, palette.text, cx, y + rowHeight / 2, maxWidth = cellWidth - ink.dp(8f))
            }
            if (symbols) ink.line(canvas, grid.left, y + rowHeight, grid.right, y + rowHeight, palette.faint)
        }
        if (symbols) {
            for (column in 1 until columns) {
                val x = grid.left + cellWidth * column
                ink.line(canvas, x, grid.top, x, grid.bottom, palette.faint)
            }
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
        ink.icon(
            canvas, if (locked) Icon.LOCK else Icon.UNLOCK, cx, side.top + slot * 2.5f, 24f,
            if (locked) palette.brand else palette.text,
        )
        for (i in 1..2) ink.line(canvas, side.left, side.top + slot * i, side.right, side.top + slot * i, palette.faint)
    }

    private fun drawBottom(canvas: Canvas) {
        ink.fill.color = palette.functionalKey
        canvas.drawRect(bottom, ink.fill)
        val keyLeft = bottom.right - ink.dp(BOTTOM_KEY_DP)
        categories.setBounds(bottom.left, bottom.top, keyLeft, bottom.bottom)
        categories.draw(canvas)
        ink.line(canvas, keyLeft, bottom.top + ink.dp(12f), keyLeft, bottom.bottom - ink.dp(12f), palette.faint)
        val cx = (keyLeft + bottom.right) / 2
        if (symbols) {
            ink.icon(canvas, Icon.KEYBOARD, cx, bottom.centerY(), 24f, palette.text)
        } else {
            keyPaint.color = palette.text
            KeyIcons.draw(canvas, Key.Backspace, cx, bottom.centerY(), ink.dp(24f), keyPaint, false)
        }
    }

    override fun onTap(x: Float, y: Float) {
        when {
            !symbols && y < headerHeight -> top.indexAt(x)?.let { chooseTop(it) }

            y > bottom.top && x > bottom.right - ink.dp(BOTTOM_KEY_DP) ->
                if (symbols) actions.closePanel() else actions.deleteBackward()

            y > bottom.top -> categories.indexAt(x)?.let {
                categories.selected = it
                if (symbols) symbolSection = it else emojiSection = it
                gridScroll.reset()
            }

            symbols && side.contains(x, y) -> when (((y - side.top) / (side.height() / 3)).toInt()) {
                0 -> actions.closePanel()
                1 -> actions.deleteBackward()
                else -> locked = !locked
            }

            grid.contains(x, y) -> tapGrid(x, y)
        }
    }

    private fun chooseTop(index: Int) {
        if (index == top.selected) return
        top.selected = index
        emojiTop = index
        categories.labels = sections().map { it.name }
        categories.selected = 0
        emojiSection = 0
        gridScroll.reset()
    }

    /** 落在网格哪一格：条目与它的格子（视图坐标）。 */
    private fun itemAt(x: Float, y: Float, cell: RectF): String? {
        if (!grid.contains(x, y)) return null
        val columns = columns()
        val rowHeight = rowHeight(columns)
        val cellWidth = grid.width() / columns
        val row = ((y - grid.top + gridScroll.offset) / rowHeight).toInt()
        val column = ((x - grid.left) / cellWidth).toInt().coerceIn(0, columns - 1)
        val item = section?.items?.getOrNull(row * columns + column) ?: return null
        val top = grid.top + row * rowHeight - gridScroll.offset
        cell.set(grid.left + cellWidth * column, top, grid.left + cellWidth * (column + 1), top + rowHeight)
        return item
    }

    private fun tapGrid(x: Float, y: Float) {
        itemAt(x, y, scratch)?.let(::commit)
    }

    private fun commit(item: String) {
        actions.commitText(item)
        Catalog.recordUse(context, item)
        if (symbols && !locked) actions.closePanel()
    }

    /** 手指落下就在格子上方放大显示这一格：手指挡住的看得见。 */
    private fun press(x: Float, y: Float) {
        pressed = itemAt(x, y, pressedCell)
        downX = x
        downY = y
        invalidate()
    }

    private fun release() {
        if (pressed == null) return
        pressed = null
        invalidate()
    }

    /** 长按：弹出这一格的气泡，能接肤色的 emoji 排出五档，滑到哪个抬手上屏哪个。 */
    override fun onHold(x: Float, y: Float, rawX: Float) {
        val item = pressed ?: return
        Haptics.hold(context)
        holdRawX = rawX
        tracking = false
        choices = listOf(item) + if (symbols) emptyList() else Catalog.skinTones(item)
        popup.show(
            choices.map { ChoicePopup.Item(it) }, initial = 0,
            fingerScreenX = screenCentreX(), bottom = pressedCell.top, centred = true,
        )
        invalidate()
    }

    /** 按住那一格的中线在屏幕上的 x：气泡对着格子居中，不对着手指偏到格子边上的位置。 */
    private fun screenCentreX(): Float {
        getLocationOnScreen(location)
        return location[0] + pressedCell.centerX()
    }

    @SuppressLint("ClickableViewAccessibility")
    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (popup.showing) {
            when (event.actionMasked) {
                MotionEvent.ACTION_MOVE -> {
                    // 长按时手指难免抖一点，横向挪开一段才开始选，不然没动就跳到别的肤色上
                    if (abs(event.rawX - holdRawX) > ink.dp(SLOP_DP)) tracking = true
                    if (tracking) popup.track(event.rawX, cancelled = event.y > pressedCell.bottom + pressedCell.height())
                }

                MotionEvent.ACTION_UP -> {
                    popup.finish()?.let { commit(choices[it]) }
                    release()
                }

                MotionEvent.ACTION_CANCEL -> {
                    popup.finish()
                    release()
                }
            }
            return true
        }
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> press(event.x, event.y)
            MotionEvent.ACTION_MOVE ->
                if (hypot(event.x - downX, event.y - downY) > ink.dp(SLOP_DP)) release()
        }
        val handled = super.onTouchEvent(event)
        if (event.actionMasked == MotionEvent.ACTION_UP || event.actionMasked == MotionEvent.ACTION_CANCEL) release()
        return handled
    }

    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()
        if (popup.showing) popup.finish()
    }

    private fun drawPressed(canvas: Canvas) {
        val item = pressed ?: return
        if (popup.showing) return
        val grow = pressedCell.width() * 0.15f
        val height = pressedCell.height() * 1.15f
        val bottom = pressedCell.top - ink.dp(2f)
        bubble.set(
            (pressedCell.left - grow).coerceAtLeast(0f), (bottom - height).coerceAtLeast(0f),
            (pressedCell.right + grow).coerceAtMost(width.toFloat()), bottom.coerceAtLeast(height),
        )
        bubble.offset(0f, ink.density)
        ink.round(canvas, bubble, 12f, palette.keyShadow)
        bubble.offset(0f, -ink.density)
        ink.round(canvas, bubble, 12f, palette.surface)
        ink.text(
            canvas, item, if (symbols) 26f else 36f, palette.text, bubble.centerX(), bubble.centerY(),
            maxWidth = bubble.width() - ink.dp(8f),
        )
    }

    private companion object {
        const val BOTTOM_DP = 52f
        const val BOTTOM_KEY_DP = 60f

        /** 手指移出这么远就算拖动，收起放大的格子。 */
        const val SLOP_DP = 10f

        /** 符号面板右边功能列占整宽的几分之一。 */
        const val SIDE_FRACTION = 5.5f

        // 面板每次打开都新建，停在哪一类、锁没锁记在这里
        var symbolSection = 0
        var emojiTop = 0
        var emojiSection = 0
        var locked = false
    }
}
