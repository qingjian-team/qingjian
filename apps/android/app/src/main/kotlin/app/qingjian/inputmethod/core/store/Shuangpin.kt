package app.qingjian.inputmethod.core.store

import app.qingjian.inputmethod.core.Native
import org.json.JSONArray

/** 双拼方案与键帽都从 Core 的方案表取，壳不写死键位；取过的方案缓存着。 */
object Shuangpin {
    val choices: List<ShuangpinChoice> by lazy {
        val json = runCatching { Native.shuangpinChoices() }.getOrNull() ?: return@lazy emptyList()
        val array = JSONArray(json)
        List(array.length()) { i ->
            val choice = array.getJSONObject(i)
            ShuangpinChoice(choice.getString("key"), choice.getString("label"))
        }
    }

    private val cache = HashMap<String, Map<Char, ShuangpinKeycap>>()

    /** [scheme] 每个键的键帽；用到 `;` 的方案多一个 `;` 键。取不到为空。 */
    fun keycaps(scheme: String): Map<Char, ShuangpinKeycap> = cache.getOrPut(scheme) {
        val json = runCatching { Native.shuangpinKeycaps(scheme) }.getOrNull() ?: return@getOrPut emptyMap()
        val array = JSONArray(json)
        (0 until array.length()).associate { i ->
            val keycap = array.getJSONObject(i)
            val finals = keycap.getJSONArray("finals")
            keycap.getString("key").first() to ShuangpinKeycap(
                keycap.getString("initial"),
                List(finals.length(), finals::getString).joinToString(" "),
            )
        }
    }
}
