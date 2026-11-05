package app.qingjian.inputmethod.candidates

import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Rect
import android.text.TextPaint
import app.qingjian.inputmethod.core.frame.Candidate
import app.qingjian.inputmethod.core.strip.StripImage
import app.qingjian.inputmethod.panel.base.PanelIcons

/**
 * 候选条的内容：Rust 渲染器按主题画好的位图（拼音行 + 横排候选 + 译文），这里只贴图。
 * 拼音行那一截钉在视口上，候选那一截随横向滚动；emoji 候选渲染器画不出，在词的位置上用 Canvas 补；
 * 通讯录里的名字在词的右上角补一个人头（各主题都有，不用每个主题自己画）。
 */
class CandidateStrip(private val density: Float, private val barHeight: Float, textScale: Float, markColor: Int) {
    private val markPaint = Paint(Paint.ANTI_ALIAS_FLAG).also { it.color = markColor }
    private val emojiPaint = TextPaint(Paint.ANTI_ALIAS_FLAG).also {
        it.textSize = 19f * density * textScale
        it.textAlign = Paint.Align.CENTER
    }
    private val fill = Paint()
    private val src = Rect()
    private val dst = Rect()

    private var preedit = ""
    private var image: StripImage? = null

    /** 当前一帧的候选，长按气泡要它们的译文。 */
    var candidates: List<Candidate> = emptyList()
        private set

    val isEmpty: Boolean get() = preedit.isEmpty() && candidates.isEmpty()

    /** 主题底色；还没画过为透明。 */
    val background: Int get() = image?.background ?: 0

    /** 拼音行与候选行的分界（位图像素）。 */
    val candidatesTop: Int get() = image?.candidatesTop ?: 0

    /** 位图宽（像素），比视口宽时可以横向滚动。 */
    val contentWidth: Float get() = image?.bitmap?.width?.toFloat() ?: 0f

    fun show(preedit: String, candidates: List<Candidate>) {
        this.preedit = preedit
        this.candidates = candidates
    }

    /** 换上新画的一帧；画不出来（null）时只铺底色。 */
    fun paint(image: StripImage?) {
        this.image = image
    }

    fun indexAt(contentX: Float): Int = image?.candidateAt(contentX) ?: -1

    /** 点在拼音行上（视图坐标，拼音行不随候选滚动）：光标该落在第几个字之后；不在拼音行上为 -1。 */
    fun caretAt(x: Float, y: Float): Int = image?.caretAt(x, y) ?: -1

    /** 拖动中：只按横坐标找最近的缝，手指上下偏出拼音行也照样跟。 */
    fun caretNear(x: Float): Int = image?.caretNear(x) ?: -1

    /** [scrollX] 是视图已经滚过去的距离，画布已经按它平移过。 */
    fun draw(canvas: Canvas, scrollX: Int, viewWidth: Int) {
        val image = image ?: return
        val bitmap = image.bitmap
        val top = image.candidatesTop.coerceIn(0, bitmap.height)
        fill.color = image.background
        canvas.drawRect(scrollX.toFloat(), 0f, (scrollX + viewWidth).toFloat(), barHeight, fill)
        // 没有拼音行（补全）时候选行在条里上下居中，不贴着顶
        val shift = if (top == 0) ((barHeight - bitmap.height) / 2).toInt().coerceAtLeast(0) else 0
        val right = minOf(scrollX + viewWidth, bitmap.width)
        if (right > scrollX && top < bitmap.height) {
            src.set(scrollX, top, right, bitmap.height)
            dst.set(src)
            dst.offset(0, shift)
            canvas.drawBitmap(bitmap, src, dst, null)
        }
        if (top > 0) {
            src.set(0, 0, minOf(viewWidth, bitmap.width), top)
            dst.set(scrollX, 0, scrollX + src.width(), top)
            canvas.drawBitmap(bitmap, src, dst, null)
        }
        for (index in image.emoji) {
            val word = image.wordRect(index) ?: continue
            if (word.right < scrollX || word.left > scrollX + viewWidth) continue
            val text = candidates.getOrNull(index)?.text ?: continue
            val baseline = word.centerY() + shift - (emojiPaint.ascent() + emojiPaint.descent()) / 2
            canvas.drawText(text, word.centerX(), baseline, emojiPaint)
        }
        val size = CONTACT_MARK_DP * density
        candidates.forEachIndexed { index, candidate ->
            if (!candidate.contact) return@forEachIndexed
            val word = image.wordRect(index) ?: return@forEachIndexed
            if (word.right < scrollX || word.left > scrollX + viewWidth) return@forEachIndexed
            // 格子左右各有一截内边距，人头放在右边那截里、贴着上沿
            val cx = word.right - size * 0.75f
            val cy = word.top + shift + size * 0.85f
            PanelIcons.draw(canvas, PanelIcons.Icon.USER, cx, cy, size, markPaint)
        }
    }

    private companion object {
        /** 人头标记多大（dp）：比译文字号小一点，不抢候选字。 */
        const val CONTACT_MARK_DP = 11f
    }
}
