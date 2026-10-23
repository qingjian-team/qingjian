package app.qingjian.inputmethod.panel.editor

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import android.text.Editable
import android.text.Layout
import android.text.Selection
import android.text.SpannableStringBuilder
import android.text.StaticLayout
import android.text.TextPaint
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.View
import android.view.inputmethod.BaseInputConnection
import android.view.inputmethod.InputConnection
import app.qingjian.inputmethod.core.store.PhraseItem
import app.qingjian.inputmethod.core.store.PhraseRules
import app.qingjian.inputmethod.panel.base.Ink
import app.qingjian.inputmethod.panel.base.PanelIcons.Icon
import app.qingjian.inputmethod.theme.Palette

/**
 * 加 / 改常用语：浮在键盘上方，键盘照常打字，打的字进这里而不是外面的输入框——
 * 输入法服务编辑时把 [connection] 当输入连接交给 Composer。上面 ⌄ 收起 / 标题 / 完成，中间正文（带字数），
 * 下面「输入码」（只收字母，最多 32 个，跟桌面自定义短语一样；点它切过去打码）与「目录」（点名字换下一组，「＋新建」打新分组名，新建中再点名字取消）。完成或收起都回常用语页。
 */
@SuppressLint("ViewConstructor")
class PhraseEditor(
    context: Context,
    private val palette: Palette,
    private val groups: List<String>,
    /** null 是新加。 */
    val phrase: PhraseItem?,
    group: String,
    private val onDone: (text: String, group: String, code: String) -> Unit,
    private val onCancel: () -> Unit,
    /** 正文与输入码之间换了焦点（输入码要直通字母，不过拼音）。 */
    private val onFocusCode: (Boolean) -> Unit,
) : View(context), FloatingEditor {
    private val ink = Ink(palette, resources.displayMetrics.density)
    private val text = SpannableStringBuilder(phrase?.text.orEmpty()).also { Selection.setSelection(it, it.length) }
    private val code = SpannableStringBuilder(phrase?.code.orEmpty()).also { Selection.setSelection(it, it.length) }
    private var groupIndex = groups.indexOf(group).coerceAtLeast(0)

    /** 新建分组的名字；[creatingGroup] 时用它而不是 [groups] 里选的。 */
    private val newGroup = SpannableStringBuilder()
    private var creatingGroup = false

    /** 打的字进哪一格。 */
    private var field = Field.TEXT

    private enum class Field { TEXT, CODE, GROUP }

    /** 正在打输入码。 */
    val codeFocused get() = field == Field.CODE

    override val lettersOnly get() = codeFocused

    private fun target(): SpannableStringBuilder = when (field) {
        Field.TEXT -> text
        Field.CODE -> code
        Field.GROUP -> newGroup
    }

    private fun chosenGroup(): String =
        if (creatingGroup && newGroup.isNotBlank()) newGroup.toString().trim() else groups.getOrElse(groupIndex) { "" }

    private val textPaint = TextPaint(Paint.ANTI_ALIAS_FLAG).apply { textSize = ink.dp(15.5f); color = palette.text }
    private val box = RectF()
    private val rect = RectF()

    /** 编辑时交给 Composer 的输入连接：上屏、组句、删字都落在正文或输入码里。 */
    override val connection: InputConnection = object : BaseInputConnection(this, true) {
        override fun getEditable(): Editable = target()

        override fun commitText(value: CharSequence, newCursorPosition: Int): Boolean {
            val accepted = when (field) {
                Field.TEXT -> value.take((MAX_TEXT - text.length).coerceAtLeast(0))
                Field.CODE -> filterCode(value)
                Field.GROUP -> value.filter { it != '\n' }.take((MAX_GROUP - newGroup.length).coerceAtLeast(0))
            }
            return super.commitText(accepted, newCursorPosition).also { invalidate() }
        }

        override fun setComposingText(value: CharSequence, newCursorPosition: Int): Boolean =
            super.setComposingText(if (field == Field.CODE) filterCode(value) else value, newCursorPosition).also { invalidate() }

        override fun finishComposingText(): Boolean = super.finishComposingText().also { invalidate() }

        override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean =
            super.deleteSurroundingText(beforeLength, afterLength).also { invalidate() }
    }

    /** 输入码只收字母、转小写，总长不过 [PhraseRules.maxCode]。 */
    private fun filterCode(value: CharSequence): CharSequence {
        val room = (PhraseRules.maxCode - code.length).coerceAtLeast(0)
        return value.filter { it in 'a'..'z' || it in 'A'..'Z' }.toString().lowercase().take(room)
    }

    /** Composer 不在组句时发来的键：删除删光标前一个字，回车在正文里换行。 */
    override fun onKey(keyCode: Int) {
        val target = target()
        when (keyCode) {
            KeyEvent.KEYCODE_DEL -> {
                val end = Selection.getSelectionEnd(target).coerceAtLeast(0)
                if (end > 0) target.delete(Character.offsetByCodePoints(target, end, -1), end)
            }
            KeyEvent.KEYCODE_ENTER -> if (field == Field.TEXT) target.insert(Selection.getSelectionEnd(target).coerceAtLeast(0), "\n")
        }
        invalidate()
    }

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        setMeasuredDimension(MeasureSpec.getSize(widthMeasureSpec), ink.dp(HEIGHT_DP).toInt())
    }

    override fun onDraw(canvas: Canvas) {
        canvas.drawColor(palette.board)
        val top = ink.dp(TITLE_DP) / 2
        ink.icon(canvas, Icon.EXPAND, ink.dp(28f), top, 22f, palette.text)
        ink.text(canvas, if (phrase == null) "添加常用语" else "编辑常用语", 16.5f, palette.brand, width / 2f, top, bold = true)
        ink.text(canvas, "完成", 15.5f, palette.text, width - ink.dp(34f), top, bold = true)

        box.set(ink.dp(10f), ink.dp(TITLE_DP), width - ink.dp(10f), height - ink.dp(6f))
        ink.round(canvas, box, 12f, palette.surface)
        val padding = ink.dp(14f)
        val textBottom = box.bottom - ink.dp(FIELDS_DP)
        if (field == Field.TEXT) {
            rect.set(box.left + ink.dp(4f), box.top + ink.dp(4f), box.right - ink.dp(4f), textBottom)
            ink.round(canvas, rect, 9f, palette.board)
        }
        canvas.save()
        canvas.clipRect(box.left, box.top, box.right, textBottom - ink.dp(18f))
        val layout = StaticLayout.Builder.obtain(text, 0, text.length, textPaint, (box.width() - padding * 2).toInt().coerceAtLeast(1))
            .setAlignment(Layout.Alignment.ALIGN_NORMAL)
            .build()
        // 字多了让末尾（光标处）留在框里
        val room = textBottom - ink.dp(18f) - box.top - padding
        canvas.translate(box.left + padding, box.top + padding - (layout.height - room).coerceAtLeast(0f))
        if (text.isEmpty()) {
            ink.textStart(canvas, "在这里输入常用语", 15.5f, palette.faint, 0f, ink.dp(10f), box.width())
        } else {
            layout.draw(canvas)
        }
        if (field == Field.TEXT) {
            val line = layout.getLineForOffset(text.length)
            val x = layout.getPrimaryHorizontal(text.length)
            ink.fill.color = palette.brand
            canvas.drawRect(x, layout.getLineTop(line).toFloat(), x + ink.dp(1.5f), layout.getLineBottom(line).toFloat(), ink.fill)
        }
        canvas.restore()
        ink.textStart(canvas, "${text.length}/$MAX_TEXT", 11.5f, palette.gloss, box.right - ink.dp(60f), textBottom - ink.dp(10f), box.right)

        val fieldsY = box.bottom - ink.dp(FIELDS_DP) / 2
        if (codeFocused) {
            rect.set(box.left + ink.dp(4f), fieldsY - ink.dp(16f), box.centerX(), fieldsY + ink.dp(16f))
            ink.round(canvas, rect, 9f, palette.board)
        }
        // 码长了放不下：留末尾（正在打的那头），前面用 … 代替
        val codeRoom = box.centerX() - box.left - padding - ink.measure("输入码：", 13.5f) - ink.dp(8f)
        var shownCode = code.toString().ifEmpty { "只收字母，可不填" }
        if (code.isNotEmpty() && ink.measure(shownCode, 14f) > codeRoom) {
            while (shownCode.length > 1 && ink.measure("…$shownCode", 14f) > codeRoom) shownCode = shownCode.drop(1)
            shownCode = "…$shownCode"
        }
        ink.textStart(canvas, "输入码：", 13.5f, palette.gloss, box.left + padding, fieldsY, box.centerX())
        ink.textStart(
            canvas, shownCode, 14f, if (code.isEmpty()) palette.faint else palette.text,
            box.left + padding + ink.measure("输入码：", 13.5f), fieldsY, box.centerX(),
        )
        drawGroupField(canvas, fieldsY, padding)
    }

    /** 右半格「目录」：点名字换下一组；右端「＋新建」打新分组名（打的时候高亮、带光标）。 */
    private fun drawGroupField(canvas: Canvas, y: Float, padding: Float) {
        val left = box.centerX() + padding
        if (field == Field.GROUP) {
            rect.set(box.centerX(), y - ink.dp(16f), box.right - ink.dp(4f), y + ink.dp(16f))
            ink.round(canvas, rect, 9f, palette.board)
        }
        ink.textStart(canvas, "目录：", 13.5f, palette.gloss, left, y, box.right)
        val nameLeft = left + ink.measure("目录：", 13.5f)
        if (creatingGroup) {
            val name = newGroup.toString()
            ink.textStart(canvas, name.ifEmpty { "新分组名" }, 14f, if (name.isEmpty()) palette.faint else palette.text, nameLeft, y, box.right - padding)
            if (field == Field.GROUP) {
                val x = nameLeft + (if (name.isEmpty()) 0f else ink.measure(name, 14f))
                ink.fill.color = palette.brand
                canvas.drawRect(x, y - ink.dp(9f), x + ink.dp(1.5f), y + ink.dp(9f), ink.fill)
            }
        } else {
            val newLabel = "＋新建"
            val newLeft = box.right - padding - ink.measure(newLabel, 13.5f)
            ink.textStart(canvas, "${groups.getOrElse(groupIndex) { "" }} ›", 14f, palette.text, nameLeft, y, newLeft - ink.dp(6f))
            ink.textStart(canvas, newLabel, 13.5f, palette.brand, newLeft, y, box.right)
        }
    }

    @SuppressLint("ClickableViewAccessibility")
    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (event.actionMasked != MotionEvent.ACTION_UP) return true
        val x = event.x
        val y = event.y
        when {
            y < ink.dp(TITLE_DP) && x < ink.dp(64f) -> onCancel()
            y < ink.dp(TITLE_DP) && x > width - ink.dp(72f) -> {
                connection.finishComposingText()
                onDone(text.toString(), chosenGroup(), code.toString())
            }
            y > height - ink.dp(FIELDS_DP + 6f) && x < width / 2f -> focus(Field.CODE)
            y > height - ink.dp(FIELDS_DP + 6f) && creatingGroup && field != Field.GROUP -> focus(Field.GROUP)
            y > height - ink.dp(FIELDS_DP + 6f) && !creatingGroup && x > width - ink.dp(NEW_GROUP_DP) -> {
                creatingGroup = true
                newGroup.clear()
                focus(Field.GROUP)
            }
            y > height - ink.dp(FIELDS_DP + 6f) -> {
                // 新建中再点一下：不建了，回到已有分组里轮换
                if (creatingGroup) {
                    creatingGroup = false
                    focus(Field.TEXT)
                } else {
                    groupIndex = (groupIndex + 1) % groups.size.coerceAtLeast(1)
                }
            }
            y > ink.dp(TITLE_DP) -> focus(Field.TEXT)
        }
        invalidate()
        return true
    }

    private fun focus(to: Field) {
        if (field == to) return
        connection.finishComposingText()
        val wasCode = codeFocused
        field = to
        if (wasCode != codeFocused) onFocusCode(codeFocused)
    }

    private companion object {
        const val HEIGHT_DP = 190f
        const val TITLE_DP = 48f
        const val FIELDS_DP = 44f
        const val NEW_GROUP_DP = 80f
        const val MAX_GROUP = 8
        const val MAX_TEXT = 500
    }
}
