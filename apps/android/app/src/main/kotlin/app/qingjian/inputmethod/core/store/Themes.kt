package app.qingjian.inputmethod.core.store

import android.graphics.Bitmap
import app.qingjian.inputmethod.core.Native
import java.nio.ByteBuffer
import org.json.JSONArray

/** 内置主题与设置页的预览图，都从 Rust 侧的主题库取（与 PC 同一份），壳不写死主题。 */
object Themes {
    val choices: List<ThemeChoice> by lazy {
        val json = runCatching { Native.themes() }.getOrNull() ?: return@lazy emptyList()
        val array = JSONArray(json)
        List(array.length()) { i ->
            val theme = array.getJSONObject(i)
            ThemeChoice(theme.getString("id"), theme.getString("name"))
        }
    }

    /** 主题名；不认识的 id 给缺省主题的名字。 */
    fun name(id: String): String =
        (choices.firstOrNull { it.id == id } ?: choices.firstOrNull())?.name.orEmpty()

    private val previews = HashMap<String, Bitmap?>()

    /**
     * 主题 [id] 的候选条样子（敲 nihao 的第一页），[scale] 是 dp → 像素的倍数；画过的记着。
     * 渲染器不能跨线程，只在主线程调（一张一两毫秒）。画不出为 null。
     */
    fun preview(id: String, dark: Boolean, scale: Float, textScale: Float): Bitmap? =
        previews.getOrPut("$id/$dark/$scale/$textScale") {
            val bytes = runCatching { Native.themePreview(id, dark, scale, textScale) }.getOrNull()
                ?: return@getOrPut null
            val header = ByteBuffer.wrap(bytes, 0, 8)
            val width = header.int
            val height = header.int
            if (width <= 0 || height <= 0) return@getOrPut null
            Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888).apply {
                copyPixelsFromBuffer(ByteBuffer.wrap(bytes, 8, bytes.size - 8))
            }
        }
}
