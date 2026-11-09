package app.qingjian.inputmethod.core.store

import app.qingjian.inputmethod.core.Native
import org.json.JSONObject

/** 一段文字的代码显示：[code] 时等宽、不重排；[spans] 是要上色的片段（UTF-16 起止与种类）。 */
class CodeView(val code: Boolean, val spans: List<CodeSpan>) {
    class CodeSpan(val start: Int, val end: Int, val kind: String)

    companion object {
        /** 认不出（或 Rust 出错）时当普通文字。 */
        fun of(text: String): CodeView {
            val json = runCatching { Native.codeView(text) }.getOrNull() ?: return CodeView(false, emptyList())
            val view = JSONObject(json)
            val spans = view.getJSONArray("spans")
            return CodeView(
                view.getBoolean("code"),
                List(spans.length()) { i ->
                    val span = spans.getJSONArray(i)
                    CodeSpan(span.getInt(0), span.getInt(1), span.getString(2))
                },
            )
        }
    }
}
