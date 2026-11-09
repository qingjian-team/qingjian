package app.qingjian.inputmethod.core.store

import android.content.Context
import app.qingjian.inputmethod.core.Native
import java.io.File
import org.json.JSONObject

/** 「我的」页的数字：今天、近 7 天、累计的汉字数，今天的英文词数，有输入的天数。 */
class UsageSummary(
    val todayHanzi: Long,
    val todayEnglish: Long,
    val recentHanzi: Long,
    val totalHanzi: Long,
    val days: Int,
) {
    companion object {
        private val EMPTY = UsageSummary(0, 0, 0, 0, 0)

        /** 读学习数据目录里的统计；读不了就全是 0。 */
        fun load(context: Context): UsageSummary {
            val dir = File(context.filesDir, "user")
            val json = runCatching { Native.usageStats(dir.path) }.getOrNull() ?: return EMPTY
            val root = JSONObject(json)
            val today = root.getJSONObject("today")
            return UsageSummary(
                todayHanzi = today.getLong("hanzi"),
                todayEnglish = today.getLong("english"),
                recentHanzi = root.getJSONObject("recent").getLong("hanzi"),
                totalHanzi = root.getJSONObject("total").getLong("hanzi"),
                days = root.getInt("days"),
            )
        }
    }
}
