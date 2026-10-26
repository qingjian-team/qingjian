package app.qingjian.inputmethod.core.frame

import org.json.JSONArray
import org.json.JSONObject

/** 一次操作之后要做的事：先上屏 [commit]，再按 [preedit] / [candidates] 重画；[notice] 是一句话提示。 */
data class Frame(
    val commit: String,
    /** 上屏前先删掉光标前几个字符（选算式结果时把算式换掉）。 */
    val replace: Int,
    val preedit: String,
    val notice: String,
    /** 九宫格的拼音列，最可能的在前；全键盘下为空。 */
    val pinyinChoices: List<String>,
    val candidates: List<Candidate>,
    /** 组句处在算式（`v1+2`）或码点里：数字与运算符进缓冲区，换数字键盘不顶首选。 */
    val expression: Boolean,
) {
    companion object {
        val EMPTY = Frame("", 0, "", "", emptyList(), emptyList(), false)

        fun parse(json: String?): Frame {
            if (json == null) return EMPTY
            val root = JSONObject(json)
            val candidates = root.getJSONArray("candidates").objects().map { item ->
                Candidate(
                    text = item.getString("text"),
                    senses = item.getJSONArray("senses").objects().map { sense ->
                        Sense(
                            pos = sense.optStringOrNull("pos"),
                            text = sense.getString("text"),
                            reading = sense.optStringOrNull("reading"),
                            fresh = sense.optBoolean("fresh"),
                        )
                    },
                    forgettable = item.optBoolean("forgettable"),
                    cloud = item.optBoolean("cloud"),
                )
            }
            val choices = root.getJSONArray("pinyin_choices")
            return Frame(
                root.getString("commit"), root.optInt("replace"), root.getString("preedit"), root.getString("notice"),
                List(choices.length(), choices::getString), candidates, root.optBoolean("expression"),
            )
        }

        private fun JSONArray.objects(): List<JSONObject> = List(length(), ::getJSONObject)

        private fun JSONObject.optStringOrNull(name: String): String? =
            if (isNull(name)) null else getString(name)
    }
}
