package app.qingjian.inputmethod.core.store

import app.qingjian.inputmethod.core.Native
import org.json.JSONObject

/** 常用语编辑要守的规则（缺省分组、输入码长度），从 Rust 侧取，与常用语落盘时的裁剪一致。 */
object PhraseRules {
    private val json: JSONObject by lazy {
        runCatching { Native.phraseRules() }.getOrNull()?.let(::JSONObject) ?: JSONObject()
    }

    /** 没指定分组时归的那组，总排第一。 */
    val defaultGroup: String by lazy { json.optString("default_group") }

    /** 输入码最长几个字母。 */
    val maxCode: Int by lazy { json.optInt("max_code") }
}
