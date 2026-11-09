package app.qingjian.inputmethod.core.store

import android.content.Context
import app.qingjian.inputmethod.core.BundledData
import app.qingjian.inputmethod.core.InputOptions
import app.qingjian.inputmethod.core.Native
import app.qingjian.inputmethod.settings.Settings
import java.io.File
import org.json.JSONObject

/**
 * 生词本页的内容（学习语言那门）：见过、眼熟、本周新见的数目，这门语言的等级，按 [Query] 筛出的词（最近见到的在前，最多 500 条）。
 * 学习语言关着时 [off] 为真、其余为空。
 */
class Vocabulary(
    val language: String,
    val seen: Long,
    val familiar: Long,
    val newThisWeek: Long,
    val levels: List<String>,
    val matched: Long,
    val words: List<VocabularyWord>,
) {
    val off get() = language == InputOptions.LEARNING_OFF

    /** 页面顶上的筛选（同 PC 生词本）、等级（空串不按级筛）与搜索（不分大小写，词、中文、例句里有就算）。 */
    data class Query(val filter: Filter = Filter.ALL, val level: String = "", val search: String = "")

    enum class Filter(val key: String) { ALL("all"), FRESH("fresh"), WEEK("week"), KNOWN("known") }

    companion object {
        fun load(context: Context, query: Query = Query()) = request(context, query, null)

        /** 标认识或取消，回改过之后的页。 */
        fun setKnown(context: Context, query: Query, word: String, known: Boolean) =
            request(context, query, JSONObject().put("op", "known").put("word", word).put("known", known))

        /** 删掉这个词（之后再见到从零记起），回改过之后的页。 */
        fun remove(context: Context, query: Query, word: String) =
            request(context, query, JSONObject().put("op", "remove").put("word", word))

        private fun request(context: Context, query: Query, change: JSONObject?): Vocabulary {
            val language = Settings.learningLanguage(context)
            val empty = Vocabulary(language, 0, 0, 0, emptyList(), 0, emptyList())
            val request = JSONObject()
                .put("language", language)
                .put("filter", query.filter.key)
                .put("level", query.level)
                .put("search", query.search)
                .put("change", change ?: JSONObject.NULL)
            val dataDir = BundledData.dir(context)
            val userDir = File(context.filesDir, "user")
            val json = runCatching { Native.vocabulary(dataDir.path, userDir.path, request.toString()) }.getOrNull()
                ?: return empty
            val root = JSONObject(json)
            val words = root.getJSONArray("words")
            val levels = root.getJSONArray("levels")
            return Vocabulary(
                language = language,
                seen = root.getLong("seen"),
                familiar = root.getLong("familiar"),
                newThisWeek = root.getLong("new_this_week"),
                levels = List(levels.length()) { levels.getString(it) },
                matched = root.getLong("matched"),
                words = List(words.length()) { VocabularyWord.of(words.getJSONObject(it)) },
            )
        }
    }
}
