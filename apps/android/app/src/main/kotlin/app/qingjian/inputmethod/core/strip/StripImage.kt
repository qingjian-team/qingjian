package app.qingjian.inputmethod.core.strip

import android.graphics.Bitmap
import android.graphics.RectF
import org.json.JSONObject

/**
 * Rust 渲染器画好的一帧候选条：位图 + 分界 + 可点区域。[candidatesTop] 以上是拼音行，壳钉在视口上；
 * [emoji] 里的候选渲染器画不出彩色 emoji（Android 13 起是 COLRv1），壳在 [wordRect] 上自己画。
 */
class StripImage(
    val bitmap: Bitmap,
    val candidatesTop: Int,
    /** 主题底色（ARGB），位图没铺满的地方用它填。 */
    val background: Int,
    val hits: List<StripHit>,
    val emoji: Set<Int>,
) {
    /** 横坐标 [x]（位图像素）落在哪个候选上，没有为 -1；不看纵坐标，按在拼音行上也算正下方那个候选。 */
    fun candidateAt(x: Float): Int =
        hits.firstOrNull { it.sense == null && x >= it.rect.left && x < it.rect.right }?.candidate ?: -1

    /** 第 [index] 个候选的词占的那块：整格去掉下面的译文。 */
    fun wordRect(index: Int): RectF? {
        val cell = hits.firstOrNull { it.candidate == index && it.sense == null }?.rect ?: return null
        val noteTop = hits.filter { it.candidate == index && it.sense != null }.minOfOrNull { it.rect.top }
        return RectF(cell.left, cell.top, cell.right, noteTop ?: cell.bottom)
    }

    companion object {
        fun parse(meta: JSONObject, bitmap: Bitmap): StripImage {
            val hits = meta.getJSONArray("hits")
            val emoji = meta.getJSONArray("emoji")
            return StripImage(
                bitmap = bitmap,
                candidatesTop = meta.getInt("candidates_top"),
                background = meta.getLong("background").toInt(),
                hits = List(hits.length()) { i ->
                    val hit = hits.getJSONObject(i)
                    val x = hit.getDouble("x").toFloat()
                    val y = hit.getDouble("y").toFloat()
                    StripHit(
                        candidate = hit.getInt("candidate"),
                        sense = if (hit.isNull("sense")) null else hit.getInt("sense"),
                        rect = RectF(
                            x, y,
                            x + hit.getDouble("width").toFloat(),
                            y + hit.getDouble("height").toFloat(),
                        ),
                    )
                },
                emoji = List(emoji.length(), emoji::getInt).toSet(),
            )
        }
    }
}
