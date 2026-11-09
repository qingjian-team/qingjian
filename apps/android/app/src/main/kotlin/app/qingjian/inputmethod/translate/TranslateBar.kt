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

    /** Composer 不在组句时的键：删除删光标前一个字，回车确定。 */
    override fun onKey(keyCode: Int) {
        when (keyCode) {
            KeyEvent.KEYCODE_DEL -> {
                val end = Selection.getSelectionEnd(text).coerceAtLeast(0)
                if (end > 0) text.delete(Character.offsetByCodePoints(text, end, -1), end)
                changed()
            }
            KeyEvent.KEYCODE_ENTER -> onConfirm()
        }
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
            // 字多了留末尾（光标处）
            var shown = text.toString()
            val room = textRight - textLeft - statusWidth
            if (ink.measure(shown, 15.5f) > room) {
                while (shown.length > 1 && ink.measure("…$shown", 15.5f) > room) shown = shown.drop(1)
                shown = "…$shown"
            }
            ink.textStart(canvas, shown, 15.5f, palette.text, textLeft, cy, textRight)
            drawCursor(canvas, textLeft + ink.measure(shown, 15.5f) + ink.dp(1f), cy)
            status?.let { ink.textStart(canvas, it, 12.5f, if (failed) FAILURE else palette.gloss, textRight - statusWidth + ink.dp(4f), cy, textRight) }
            ink.icon(canvas, PanelIcons.Icon.CLEAR, clearX, cy, 20f, palette.gloss)
        }
        ink.text(canvas, if (text.isEmpty()) "取消" else "确定", 16f, palette.text, buttonLeft + ink.dp(BUTTON_DP) / 2, cy, bold = true)
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
