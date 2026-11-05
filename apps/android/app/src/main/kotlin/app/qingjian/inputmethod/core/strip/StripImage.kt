package app.qingjian.inputmethod.core.strip

import android.graphics.Bitmap
import android.graphics.RectF
import kotlin.math.abs
import org.json.JSONArray
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
    /** 拼音行上各字之间那道缝的 x，第 i 个是第 i 个字之前；没有拼音行时为空。 */
    val preeditEdges: List<Float>,
    val preeditTop: Float,
    val preeditBottom: Float,
) {
    /** ([x], [y]) 落在拼音行上时光标该在第几个字之后（离哪道缝最近），不在拼音行上为 -1。 */
    fun caretAt(x: Float, y: Float): Int {
        // 拼音行的字只有十几 dp 高，候选行以上整条都算，手指不用对准字
        if (preeditEdges.isEmpty() || y >= maxOf(preeditBottom, candidatesTop.toFloat())) return -1
        return preeditEdges.indices.minBy { abs(preeditEdges[it] - x) }
    }

    /** 沿拼音行拖动时只看横坐标：离 [x] 最近的那道缝；没有拼音行为 -1。 */
    fun caretNear(x: Float): Int =
        if (preeditEdges.isEmpty()) -1 else preeditEdges.indices.minBy { abs(preeditEdges[it] - x) }

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
            val edges = meta.optJSONArray("preedit_edges") ?: JSONArray()
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
                preeditEdges = List(edges.length()) { edges.getDouble(it).toFloat() },
                preeditTop = meta.optDouble("preedit_top").toFloat(),
                preeditBottom = meta.optDouble("preedit_bottom").toFloat(),
            )
        }
    }
}
