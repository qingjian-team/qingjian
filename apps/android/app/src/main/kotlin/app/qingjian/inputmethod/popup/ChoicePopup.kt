package app.qingjian.inputmethod.popup

import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import android.text.TextPaint
import android.view.Gravity
import android.view.View
import android.widget.PopupWindow
import app.qingjian.inputmethod.theme.Palette

/**
 * 长按弹出的气泡：手指滑到哪项哪项高亮，抬手选中，滑出气泡取消。候选条（译词）、九宫格数字键（字母 / 数字）与表情用横排；
 * 中英键的语言用竖排（[show] 的 `vertical`）：语言名长、以后语言多，往上一列排得下也好滑。
 * 气泡只负责画，触摸仍由发起长按的视图收（手势是从它开始的）。
 */
class ChoicePopup(
    private val anchor: View,
    private val palette: Palette,
    /** 大字的字号（sp）；表情用大一号，看得清肤色。 */
    private val titleSp: Float = 17f,
) {
    /** 气泡里的一项：[title] 大字，[note] 是下面一行小字（可空），[fresh] 用强调色。 */
    class Item(val title: String, val note: String = "", val fresh: Boolean = false)

    /** 一项占的那一段：横排是 x 方向的起点 [start] 与宽 [size]，竖排是 y 方向的。 */
    private class Slot(val item: Item, val size: Float) {
        var start = 0f
    }

    private val context: Context = anchor.context
    private val density = context.resources.displayMetrics.density
    private val titlePaint = TextPaint(Paint.ANTI_ALIAS_FLAG).also { it.textSize = titleSp * density }
    private val notePaint = TextPaint(Paint.ANTI_ALIAS_FLAG).also { it.textSize = 11.5f * density }
    private val fill = Paint(Paint.ANTI_ALIAS_FLAG)
    private val rect = RectF()

    private val margin = 6f * density
    private val itemPadding = (if (titleSp > 20f) 6f else 16f) * density
    private val minItemWidth = maxOf(40f, titleSp * 1.8f) * density

    private var slots: List<Slot> = emptyList()
    private var highlighted = -1
    private var vertical = false

    /** 气泡左上角在屏幕上的位置。 */
    private var screenLeft = 0f
    private var screenTop = 0f

    private val content = object : View(context) {
        override fun onDraw(canvas: Canvas) = draw(canvas, width.toFloat(), height.toFloat())
    }

    private val window = PopupWindow(content).apply {
        isTouchable = false
        isFocusable = false
        isClippingEnabled = false
        elevation = 0f
    }

    val showing: Boolean get() = window.isShowing

    /**
     * 弹在 [anchor] 里 y = [bottom] 这条线的上方，第 [initial] 项对着手指（长按不滑直接抬手就选它；-1 是不预选、第一项对着手指，得滑到某项上才算）；
     * [centred] 时改成整串居中对着手指（表情的肤色两边都能滑到）。贴到屏幕边就往里收。
     * [vertical] 时竖排成一列、居中对着手指，手指往上滑选。
     */
    fun show(
        items: List<Item>,
        initial: Int,
        fingerScreenX: Float,
        bottom: Float,
        centred: Boolean = false,
        vertical: Boolean = false,
    ) {
        this.vertical = vertical
        highlighted = initial
        if (vertical) {
            showColumn(items, fingerScreenX, bottom)
            return
        }
        slots = items.map { item ->
            val text = maxOf(titlePaint.measureText(item.title), notePaint.measureText(item.note))
            Slot(item, maxOf(text + itemPadding * 2, minItemWidth))
        }
        var x = margin
        for (slot in slots) {
            slot.start = x
            x += slot.size
        }
        val width = x + margin
        val height = (if (items.any { it.note.isNotEmpty() }) 64f else maxOf(52f, titleSp * 2.4f)) * density
        val target = slots[initial.coerceAtLeast(0)]
        val offset = if (centred) width / 2 else target.start + target.size / 2
        screenLeft = clampLeft(fingerScreenX - offset, width)
        place(width, height, bottom)
    }

    /** 竖排：每项一行、同宽，整列居中对着手指，底边贴着 [bottom]。 */
    private fun showColumn(items: List<Item>, fingerScreenX: Float, bottom: Float) {
        val rowHeight = ROW_HEIGHT_DP * density
        slots = items.map { Slot(it, rowHeight) }
        slots.forEachIndexed { i, slot -> slot.start = margin + i * rowHeight }
        val width = maxOf(items.maxOf { titlePaint.measureText(it.title) } + itemPadding * 2, minItemWidth) + margin * 2
        val height = margin * 2 + rowHeight * items.size
        screenLeft = clampLeft(fingerScreenX - width / 2, width)
        place(width, height, bottom)
    }

    private fun clampLeft(left: Float, width: Float): Float {
        val screenWidth = context.resources.displayMetrics.widthPixels
        return left.coerceIn(margin, (screenWidth - width - margin).coerceAtLeast(margin))
    }

    private fun place(width: Float, height: Float, bottom: Float) {
        val location = IntArray(2)
        anchor.getLocationInWindow(location)
        val top = bottom - height - 2 * density
        window.width = width.toInt()
        window.height = height.toInt()
        window.showAtLocation(anchor, Gravity.NO_GRAVITY, screenLeft.toInt(), (location[1] + top).toInt())
        // 摆放按窗口坐标，跟手按屏幕坐标：输入法窗口不在屏幕顶上，两者差一截
        anchor.getLocationOnScreen(location)
        screenTop = location[1] + top
    }

    /**
     * 手指移动：横排看横向落在哪项，滑出左右两端或 [cancelled] 就是取消；竖排看 [fingerScreenY] 落在哪一行，
     * 手指还在气泡下面（按着的键上）时留着原来的高亮，往上滑过顶停在第一项，横着偏出气泡太远取消。
     */
    fun track(fingerScreenX: Float, cancelled: Boolean, fingerScreenY: Float = 0f) {
        val index = when {
            cancelled -> -1
            vertical -> trackColumn(fingerScreenX, fingerScreenY)
            else -> {
                val x = fingerScreenX - screenLeft
                slots.indexOfFirst { x >= it.start && x < it.start + it.size }
            }
        }
        if (index != highlighted) {
            highlighted = index
            content.invalidate()
        }
    }

    private fun trackColumn(fingerScreenX: Float, fingerScreenY: Float): Int {
        val x = fingerScreenX - screenLeft
        val slop = SIDE_SLOP_DP * density
        if (x < -slop || x > window.width + slop) return -1
        val y = fingerScreenY - screenTop
        return when {
            y >= window.height -> highlighted
            y < 0 -> 0
            else -> slots.indexOfFirst { y >= it.start && y < it.start + it.size }.takeIf { it >= 0 }
                ?: if (y < margin) 0 else slots.lastIndex
        }
    }

    /** 抬手：返回选中的是第几项（取消为 null），收起气泡。 */
    fun finish(): Int? {
        val choice = highlighted.takeIf { it in slots.indices }
        window.dismiss()
        return choice
    }

    private fun draw(canvas: Canvas, width: Float, height: Float) {
        val radius = 12f * density
        fill.color = palette.keyShadow
        canvas.drawRoundRect(0f, density, width, height, radius, radius, fill)
        fill.color = palette.surface
        canvas.drawRoundRect(0f, 0f, width, height - density, radius, radius, fill)
        if (vertical) {
            drawColumn(canvas, width)
            return
        }

        slots.forEachIndexed { index, slot ->
            if (index == highlighted) {
                rect.set(slot.start, margin, slot.start + slot.size, height - margin - density)
                fill.color = palette.accent
                canvas.drawRoundRect(rect, 8 * density, 8 * density, fill)
            }
            val centre = slot.start + slot.size / 2
            val item = slot.item
            titlePaint.textAlign = Paint.Align.CENTER
            titlePaint.color = if (item.fresh) palette.fresh else palette.text
            val titleBaseline = if (item.note.isEmpty()) height / 2 + titleSp * 0.35f * density else 29 * density
            canvas.drawText(item.title, centre, titleBaseline, titlePaint)
            if (item.note.isNotEmpty()) {
                notePaint.textAlign = Paint.Align.CENTER
                notePaint.color = palette.gloss
                canvas.drawText(item.note, centre, 48 * density, notePaint)
            }
        }
    }

    /** 竖排：每行居中写一项，高亮整行。 */
    private fun drawColumn(canvas: Canvas, width: Float) {
        titlePaint.textAlign = Paint.Align.CENTER
        slots.forEachIndexed { index, slot ->
            if (index == highlighted) {
                rect.set(margin, slot.start, width - margin, slot.start + slot.size)
                fill.color = palette.accent
                canvas.drawRoundRect(rect, 8 * density, 8 * density, fill)
            }
            titlePaint.color = if (slot.item.fresh) palette.fresh else palette.text
            val baseline = slot.start + slot.size / 2 + titleSp * 0.35f * density
            canvas.drawText(slot.item.title, width / 2, baseline, titlePaint)
        }
    }

    private companion object {
        /** 竖排一行的高。 */
        const val ROW_HEIGHT_DP = 46f

        /** 竖排时手指横着偏出气泡这么远才算取消（手指往上滑难免歪）。 */
        const val SIDE_SLOP_DP = 24f
    }
}
