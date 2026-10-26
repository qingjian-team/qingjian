package app.qingjian.inputmethod.core.store

import android.content.Context
import app.qingjian.inputmethod.core.Native
import java.io.File
import org.json.JSONArray

/** Core 编进程序的面板目录，第一次打开面板时从 Rust 取一次，之后常驻。 */
object Catalog {
    const val SYMBOLS = "符号"
    const val EMOJI = "表情"

    /** 表情组里颜文字那一类：在面板顶上单独成一页。 */
    const val KAOMOJI = "颜文字"

    val sections: List<CatalogSection> by lazy {
        val json = runCatching { Native.panelCatalog() }.getOrNull() ?: return@lazy emptyList()
        val array = JSONArray(json)
        List(array.length()) { i ->
            val section = array.getJSONObject(i)
            val items = section.getJSONArray("items")
            CatalogSection(section.getString("group"), section.getString("name"), List(items.length(), items::getString))
        }
    }

    /** 「常用」分类的名字。 */
    const val COMMON = "常用"

    /** [group] 组「常用」该显示的条目，用户用过的在前；取不到返回 null（用内置的）。每次打开面板时取。 */
    fun common(context: Context, group: String): List<String>? {
        val json = runCatching { Native.panelCommon(userDir(context), group) }.getOrNull() ?: return null
        val array = JSONArray(json)
        return List(array.length(), array::getString).ifEmpty { null }
    }

    /** 面板里点了一格：记一次，下次打开时排进「常用」。 */
    fun recordUse(context: Context, item: String) {
        runCatching { Native.recordPanelUse(userDir(context), item) }
    }

    private fun userDir(context: Context) = File(context.filesDir, "user").path

    /** 五档肤色，浅到深；不能接肤色的返回空。 */
    fun skinTones(emoji: String): List<String> {
        val json = runCatching { Native.skinTones(emoji) }.getOrNull() ?: return emptyList()
        val array = JSONArray(json)
        return List(array.length(), array::getString)
    }

    fun group(group: String): List<CatalogSection> = sections.filter { it.group == group }
}
