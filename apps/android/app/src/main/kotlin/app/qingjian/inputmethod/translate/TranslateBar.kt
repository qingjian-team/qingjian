package app.qingjian.inputmethod.translate

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.RectF
import android.text.Editable
import android.text.Selection
import android.text.SpannableStringBuilder
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.View
import android.view.inputmethod.BaseInputConnection
import android.view.inputmethod.InputConnection
import app.qingjian.inputmethod.panel.base.Ink
import app.qingjian.inputmethod.panel.base.PanelIcons
import app.qingjian.inputmethod.panel.editor.FloatingEditor
import app.qingjian.inputmethod.theme.Palette

/**
 * 快捷翻译：浮在工具栏上方的一条，左边「中⇌英」、中间翻译框、右边 取消 / 确定。
 * 键盘打的字进翻译框；原文（不含还在组句的拼音）一变就交给 [onSource]，服务那边停顿后发云端，
 * 译文以组合文字（带下划线、未确认）放进外面的输入框，改原文就跟着换。确定（或回车）确认上屏，取消把它撤掉。
 * 方向由 Core 按原文定：汉字为主译成学习语言，外文译成中文。
 */
@SuppressLint("ViewConstructor")
class TranslateBar(
    context: Context,
    private val palette: Palette,
    /** 学习语言的简称（英、日），画在方向胶囊上。 */
    private val partner: String,
    private val onSource: (String) -> Unit,
    private val onConfirm: () -> Unit,
    private val onCancel: () -> Unit,
    /** 点框里挪了光标：Composer 收掉组句。 */
    private val onCursorMoved: () -> Unit = {},
) : View(context), FloatingEditor {
    private val ink = Ink(palette, resources.displayMetrics.density)
    private val text = SpannableStringBuilder().also { Selection.setSelection(it, 0) }
    private val box = RectF()
    private val chip = RectF()

    /** 翻译框右侧的状态（翻译中…、翻译失败）；null 不显示。 */
    var status: String? = null
        set(value) {
            field = value
            invalidate()
        }

    /** [status] 是失败提示：红字，与翻译中的灰字分开，免得当成译文。 */
    var failed = false
        set(value) {
            field = value
            invalidate()
        }

    /** 上次画出来的那段：从原文第几个字起、画在哪个横坐标，点框里时按它算光标落在哪。 */
    private var drawnStart = 0
    private var drawnLeft = 0f

    /** 上一次交出去的原文，没变就不再交（组句中的拼音变化不算）。 */
    private var lastSource = ""

    override val enterLabel get() = "确定"

    override val connection: InputConnection = object : BaseInputConnection(this, true) {
        override fun getEditable(): Editable = text

        override fun commitText(value: CharSequence, newCursorPosition: Int): Boolean =
            super.commitText(value.take((MAX_TEXT - text.length).coerceAtLeast(0)), newCursorPosition).also { changed() }

        override fun setComposingText(value: CharSequence, newCursorPosition: Int): Boolean =
            super.setComposingText(value, newCursorPosition).also { changed() }

        override fun finishComposingText(): Boolean = super.finishComposingText().also { changed() }

        override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean =
            super.deleteSurroundingText(beforeLength, afterLength).also { changed() }
    }

    /** Composer 不在组句时的键：删除删光标前一个字，回车确定，左右方向键（按住空格左右滑）挪光标。 */
    override fun onKey(keyCode: Int) {
        val end = caret()
        when (keyCode) {
            KeyEvent.KEYCODE_DEL -> {
                if (end > 0) text.delete(Character.offsetByCodePoints(text, end, -1), end)
                changed()
            }
            KeyEvent.KEYCODE_ENTER -> onConfirm()
            KeyEvent.KEYCODE_DPAD_LEFT -> if (end > 0) moveTo(Character.offsetByCodePoints(text, end, -1))
            KeyEvent.KEYCODE_DPAD_RIGHT -> if (end < text.length) moveTo(Character.offsetByCodePoints(text, end, 1))
        }
    }

    private fun caret() = Selection.getSelectionEnd(text).coerceIn(0, text.length)

    private fun moveTo(index: Int) {
        Selection.setSelection(text, index)
        invalidate()
    }

    /** 原文：去掉还在组句的那段拼音。 */
    private fun source(): String {
        val start = BaseInputConnection.getComposingSpanStart(text)
        val end = BaseInputConnection.getComposingSpanEnd(text)
        val plain = if (start in 0 until end) text.replaceRange(start, end, "") else text
        return plain.toString().trim()
    }

    private fun changed() {
        invalidate()
        val source = source()
        if (source == lastSource) return
        lastSource = source
        onSource(source)
    }

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        setMeasuredDimension(MeasureSpec.getSize(widthMeasureSpec), ink.dp(HEIGHT_DP).toInt())
    }

    override fun onDraw(canvas: Canvas) {
        canvas.drawColor(palette.board)
        val cy = height / 2f
        val buttonLeft = width - ink.dp(BUTTON_DP)
        box.set(ink.dp(10f), cy - ink.dp(20f), buttonLeft, cy + ink.dp(20f))
        ink.round(canvas, box, 20f, palette.surface)

        val chipLabel = "中⇌$partner"
        chip.set(box.left + ink.dp(5f), box.top + ink.dp(5f), box.left + ink.dp(14f) + ink.measure(chipLabel, 14f), box.bottom - ink.dp(5f))
        ink.round(canvas, chip, 15f, palette.brand)
        ink.text(canvas, chipLabel, 14f, palette.onBrand, chip.centerX(), cy, bold = true)

        val textLeft = chip.right + ink.dp(10f)
        val clearX = box.right - ink.dp(CLEAR_DP) / 2
        val textRight = if (text.isEmpty()) box.right - ink.dp(10f) else clearX - ink.dp(CLEAR_DP) / 2
        val statusWidth = status?.let { ink.measure(it, 12.5f) + ink.dp(8f) } ?: 0f
        if (text.isEmpty()) {
            ink.textStart(canvas, "输入要翻译的中/英文", 15.5f, palette.faint, textLeft + ink.dp(4f), cy, textRight)
            drawCursor(canvas, textLeft, cy)
        } else {
            drawnLeft = textLeft
            val (shown, beforeCaret) = visible(textRight - textLeft - statusWidth)
            ink.textStart(canvas, shown, 15.5f, palette.text, textLeft, cy, textRight)
            drawCursor(canvas, textLeft + ink.measure(beforeCaret, 15.5f) + ink.dp(1f), cy)
            status?.let { ink.textStart(canvas, it, 12.5f, if (failed) FAILURE else palette.gloss, textRight - statusWidth + ink.dp(4f), cy, textRight) }
            ink.icon(canvas, PanelIcons.Icon.CLEAR, clearX, cy, 20f, palette.gloss)
        }
        ink.text(canvas, if (text.isEmpty()) "取消" else "确定", 16f, palette.text, buttonLeft + ink.dp(BUTTON_DP) / 2, cy, bold = true)
    }

    /**
     * 放得下的那一段与其中光标前的部分：字多了先从开头省（光标在末尾时留末尾），光标挪到前面放不下了再从末尾省，
     * 省掉的一头画「…」。
     */
    private fun visible(room: Float): Pair<String, String> {
        val all = text.toString()
        val caret = caret()
        var start = 0
        var end = all.length
        fun label(from: Int, to: Int) = (if (from > 0) "…" else "") + all.substring(from, to) + (if (to < all.length) "…" else "")
        while (start < caret && ink.measure(label(start, end), 15.5f) > room) start = all.offsetByCodePoints(start, 1)
        while (end > caret && ink.measure(label(start, end), 15.5f) > room) end = all.offsetByCodePoints(end, -1)
        val shown = label(start, end)
        drawnStart = start
        return shown to (if (start > 0) "…" else "") + all.substring(start, caret)
    }

    /** 光标挪到手指处：画出来那段里离手指最近的字缝。 */
    private fun placeCursor(x: Float) {
        onCursorMoved()
        connection.finishComposingText()
        val all = text.toString()
        val lead = if (drawnStart > 0) "…" else ""
        var best = drawnStart
        var index = drawnStart
        while (index < all.length) {
            val next = all.offsetByCodePoints(index, 1)
            val mid = (ink.measure(lead + all.substring(drawnStart, index), 15.5f) + ink.measure(lead + all.substring(drawnStart, next), 15.5f)) / 2
            if (drawnLeft + mid > x) break
            best = next
            index = next
        }
        moveTo(best)
    }

    private fun drawCursor(canvas: Canvas, x: Float, cy: Float) {
        ink.fill.color = palette.brand
        canvas.drawRect(x, cy - ink.dp(10f), x + ink.dp(1.5f), cy + ink.dp(10f), ink.fill)
    }

    @SuppressLint("ClickableViewAccessibility")
    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (event.actionMasked != MotionEvent.ACTION_UP) return true
        val x = event.x
        when {
            x > width - ink.dp(BUTTON_DP) -> if (text.isEmpty()) onCancel() else onConfirm()
            text.isNotEmpty() && x > box.right - ink.dp(CLEAR_DP) && x < box.right -> {
                connection.finishComposingText()
                text.clear()
                changed()
            }
            text.isNotEmpty() && x > drawnLeft && box.contains(x, event.y) -> placeCursor(x)
        }
        return true
    }

    private companion object {
        const val HEIGHT_DP = 54f
        const val BUTTON_DP = 72f
        const val CLEAR_DP = 36f
        const val MAX_TEXT = 500

        /** 失败提示的红字。 */
        const val FAILURE = 0xFFD93B3B.toInt()
    }
}
