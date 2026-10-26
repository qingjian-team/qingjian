package app.qingjian.inputmethod.core.store

import android.content.Context
import app.qingjian.inputmethod.core.Native
import java.io.File
import org.json.JSONArray
import org.json.JSONObject

/**
 * 剪贴板历史与常用语的读写，都在 Rust 侧（`qingjian_learning` 的 `ClipboardHistory` / `PhraseBook`），这里只拼请求、解结果。
 * 不要引擎句柄，App 与输入法都能调；出错返回空的。
 */
object ClipboardStore {
    fun userDir(context: Context): String = File(context.filesDir, "user").apply { mkdirs() }.path

    fun list(context: Context, limit: Int) = clipboard(context, JSONObject().put("op", "list"), limit)

    fun add(context: Context, text: String, limit: Int) =
        clipboard(context, JSONObject().put("op", "add").put("text", text), limit)

    fun addImage(context: Context, bytes: ByteArray, extension: String, limit: Int): List<ClipItem> =
        parseClips(runCatching { Native.clipboardAddImage(userDir(context), bytes, extension, limit) }.getOrNull())

    fun remove(context: Context, index: Int, limit: Int) =
        clipboard(context, JSONObject().put("op", "remove").put("index", index), limit)

    fun clear(context: Context, limit: Int) = clipboard(context, JSONObject().put("op", "clear"), limit)

    private fun clipboard(context: Context, request: JSONObject, limit: Int): List<ClipItem> =
        parseClips(runCatching { Native.clipboard(userDir(context), request.toString(), limit) }.getOrNull())

    private fun parseClips(json: String?): List<ClipItem> {
        json ?: return emptyList()
        val array = JSONArray(json)
        return List(array.length()) { i ->
            val item = array.getJSONObject(i)
            ClipItem(item.optString("text"), item.optString("image").takeIf { !item.isNull("image") && it.isNotEmpty() })
        }
    }

    fun phrases(context: Context) = phrases(context, JSONObject().put("op", "list"))

    fun addPhrase(context: Context, text: String, group: String, code: String) =
        phrases(context, JSONObject().put("op", "add").put("text", text).put("group", group).put("code", code))

    fun updatePhrase(context: Context, index: Int, text: String, group: String, code: String) = phrases(
        context,
        JSONObject().put("op", "update").put("index", index).put("text", text).put("group", group).put("code", code),
    )

    fun removePhrase(context: Context, index: Int) = phrases(context, JSONObject().put("op", "remove").put("index", index))

    fun phraseToTop(context: Context, index: Int) = phrases(context, JSONObject().put("op", "top").put("index", index))

    private fun phrases(context: Context, request: JSONObject): PhrasePage {
        val json = runCatching { Native.phrases(userDir(context), request.toString()) }.getOrNull()
            ?: return PhrasePage(listOf(PhraseRules.defaultGroup), emptyList())
        val page = JSONObject(json)
        val groups = page.getJSONArray("groups")
        val phrases = page.getJSONArray("phrases")
        return PhrasePage(
            List(groups.length(), groups::getString),
            List(phrases.length()) { i ->
                val phrase = phrases.getJSONObject(i)
                PhraseItem(i, phrase.getString("text"), phrase.getString("group"), phrase.optString("code"))
            },
        )
    }
}
