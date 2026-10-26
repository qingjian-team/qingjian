package app.qingjian.inputmethod.core.store

import org.json.JSONObject

/** 生词本里的一个词：译词、带它上屏的中文与那一句、几个次数、最近见到的日期、认识没有、还算不算生词、等级（没有为空串）。 */
class VocabularyWord(
    val word: String,
    val source: String,
    val example: String,
    val seen: Int,
    val committed: Int,
    val used: Int,
    val last: String,
    val known: Boolean,
    val fresh: Boolean,
    val level: String,
) {
    companion object {
        fun of(json: JSONObject) = VocabularyWord(
            word = json.getString("word"),
            source = json.getString("source"),
            example = json.getString("example"),
            seen = json.getInt("seen"),
            committed = json.getInt("committed"),
            used = json.getInt("used"),
            last = json.getString("last"),
            known = json.getBoolean("known"),
            fresh = json.getBoolean("fresh"),
            level = json.getString("level"),
        )
    }
}
