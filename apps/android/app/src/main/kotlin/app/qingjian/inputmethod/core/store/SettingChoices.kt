package app.qingjian.inputmethod.core.store

import app.qingjian.inputmethod.core.Native
import org.json.JSONArray
import org.json.JSONObject

/** 键盘设置页的选项清单与缺省值（模糊音、学习语言、繁体字形、领域词库、剪贴板条数），都从 Rust 侧取（名字与 PC 设置程序同一份），取一次缓存着。 */
object SettingChoices {
    private val json: JSONObject by lazy {
        runCatching { Native.settingChoices() }.getOrNull()?.let(::JSONObject) ?: JSONObject()
    }

    /** 模糊音规则，按 PC 设置程序的顺序。 */
    val fuzzy: List<Choice> by lazy { choices("fuzzy") }

    /** 学习语言，最后一项是不显示译文。 */
    val learningLanguages: List<Choice> by lazy { choices("learning_languages") }

    val defaultLearningLanguage: String by lazy { json.optString("default_learning_language") }

    /** 功能面板「繁体」开着时的字形。 */
    val traditionalVariants: List<Choice> by lazy { choices("traditional_variants") }

    val defaultTraditionalVariant: String by lazy { json.optString("default_traditional_variant") }

    /** 缺省打开的随包领域词库（文件名不含扩展名）。 */
    val defaultDomains: Set<String> by lazy {
        val array = json.optJSONArray("default_domains") ?: JSONArray()
        List(array.length(), array::getString).toSet()
    }

    /** 剪贴板最多存多少条的几档。 */
    val clipboardLimits: List<Int> by lazy {
        val array = json.optJSONArray("clipboard_limits") ?: JSONArray()
        List(array.length(), array::getInt)
    }

    val defaultClipboardLimit: Int by lazy { json.optInt("default_clipboard_limit", 1) }

    /** 学习语言的名字；不认识的代码按缺省语言。 */
    fun learningLanguageName(code: String) = label(learningLanguages, code, defaultLearningLanguage)

    /** 繁体字形的名字；不认识的写法按缺省字形。 */
    fun traditionalVariantName(key: String) = label(traditionalVariants, key, defaultTraditionalVariant)

    private fun label(choices: List<Choice>, key: String, fallback: String): String =
        (choices.firstOrNull { it.key == key } ?: choices.firstOrNull { it.key == fallback })?.label.orEmpty()

    private fun choices(name: String): List<Choice> {
        val array = json.optJSONArray(name) ?: return emptyList()
        return List(array.length()) { i ->
            val choice = array.getJSONObject(i)
            Choice(choice.getString("key"), choice.getString("label"))
        }
    }
}
