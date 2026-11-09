package app.qingjian.inputmethod.core.store

import android.content.Context
import app.qingjian.inputmethod.core.BundledData
import app.qingjian.inputmethod.core.Native
import app.qingjian.inputmethod.settings.Settings
import java.io.File
import org.json.JSONObject

/**
 * 统计详情页的数（Rust `StatsDetail`）：今天 / 近 7 天 / 本月 / 累计（口径与 PC 同一套，见 Rust `qingjian_learning::figures`），过去 24 小时与最近 30 天的汉字数，
 * 几个算出来的指标（打字太少时为 null），学习语言的词汇汇总（关着为 null）。
 */
class StatsDetail(
    val today: Count,
    val recent: Count,
    val month: Count,
    val total: Count,
    val hours: List<Long>,
    val hour: Int,
    val days: List<Day>,
    val bestDay: Day?,
    val activeDays: Long,
    val average: Long,
    val todaySpeed: Double?,
    val averageSpeed: Double?,
    val firstChoiceRate: Double?,
    val cleanRate: Double?,
    val keysPerHanzi: Double?,
    val vocabulary: Vocabulary?,
) {
    class Count(val hanzi: Long, val english: Long)

    /** [date] 是 `2026-10-19`。 */
    class Day(val date: String, val hanzi: Long)

    class Level(val name: String, val total: Long, val seen: Long, val familiar: Long, val committed: Long)

    class Vocabulary(
        val language: String,
        val seen: Long,
        val familiar: Long,
        val committed: Long,
        val used: Long,
        val newThisWeek: Long,
        val levels: List<Level>,
    )

    companion object {
        /**
         * 读不了返回 null，页面显示成没有记录。[vocabulary] 为 false 时不读生词本（首页看板用不着，省一次读文件）。
         */
        fun load(context: Context, vocabulary: Boolean = true): StatsDetail? {
            val language = if (vocabulary) Settings.learningLanguage(context) else "off"
            val json = runCatching {
                Native.statsDetail(BundledData.dir(context).path, File(context.filesDir, "user").path, language)
            }.getOrNull() ?: return null
            return parse(JSONObject(json))
        }

        private fun parse(root: JSONObject): StatsDetail {
            fun count(key: String) = root.getJSONObject(key).let { Count(it.getLong("hanzi"), it.getLong("english")) }
            fun day(json: JSONObject) = Day(json.getString("date"), json.getLong("hanzi"))
            fun number(key: String) = if (root.isNull(key)) null else root.getDouble(key)
            val hours = root.getJSONArray("hours")
            val days = root.getJSONArray("days")
            return StatsDetail(
                today = count("today"),
                recent = count("recent"),
                month = count("month"),
                total = count("total"),
                hours = List(hours.length()) { hours.getLong(it) },
                hour = root.getInt("hour"),
                days = List(days.length()) { day(days.getJSONObject(it)) },
                bestDay = if (root.isNull("best_day")) null else day(root.getJSONObject("best_day")),
                activeDays = root.getLong("active_days"),
                average = root.getLong("average"),
                todaySpeed = number("today_speed"),
                averageSpeed = number("average_speed"),
                firstChoiceRate = number("first_choice_rate"),
                cleanRate = number("clean_rate"),
                keysPerHanzi = number("keys_per_hanzi"),
                vocabulary = if (root.isNull("vocabulary")) null else vocabulary(root.getJSONObject("vocabulary")),
            )
        }

        private fun vocabulary(json: JSONObject): Vocabulary {
            val levels = json.getJSONArray("levels")
            return Vocabulary(
                language = json.getString("language"),
                seen = json.getLong("seen"),
                familiar = json.getLong("familiar"),
                committed = json.getLong("committed"),
                used = json.getLong("used"),
                newThisWeek = json.getLong("new_this_week"),
                levels = List(levels.length()) {
                    val level = levels.getJSONObject(it)
                    Level(level.getString("name"), level.getLong("total"), level.getLong("seen"), level.getLong("familiar"), level.getLong("committed"))
                },
            )
        }
    }
}
