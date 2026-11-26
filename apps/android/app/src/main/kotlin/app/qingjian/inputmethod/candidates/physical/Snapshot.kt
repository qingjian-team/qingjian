//! 当前候选帧的独立快照，避免引擎复用共享位图时覆盖正在显示的内容。

package app.qingjian.inputmethod.candidates.physical

import android.graphics.Bitmap
import android.graphics.RectF
import app.qingjian.inputmethod.core.strip.StripHit
import app.qingjian.inputmethod.core.strip.StripImage
import app.qingjian.inputmethod.core.strip.StripPage

internal class Snapshot(
    val bitmap: Bitmap,

    val width: Int,

    val height: Int,

    val background: Int,

    val hits: List<StripHit>,

    val pages: List<StripPage>,

    val emoji: Set<Int>,

    val rawImage: StripImage,
) {
    fun wordRect(index: Int): RectF? = rawImage.wordRect(index)
}
