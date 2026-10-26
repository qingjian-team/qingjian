package app.qingjian.inputmethod.candidates

import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Rect
import android.text.TextPaint
import app.qingjian.inputmethod.core.frame.Candidate
import app.qingjian.inputmethod.core.strip.StripImage

/**
 * 候选条的内容：Rust 渲染器按主题画好的位图（拼音行 + 横排候选 + 译文），这里只贴图。
 * 拼音行那一截钉在视口上，候选那一截随横向滚动；emoji 候选渲染器画不出，在词的位置上用 Canvas 补。
 */
class CandidateStrip(density: Float, private val barHeight: Float, textScale: Float) {
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
    }
}
