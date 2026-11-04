package app.qingjian.inputmethod.settings.page

import android.view.Gravity
import android.view.View
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.LinearLayout
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.core.store.SettingChoices
import app.qingjian.inputmethod.core.store.StatsDetail
import app.qingjian.inputmethod.settings.widget.BarChart

/**
 * 输入统计详情（参照 PC 设置程序的统计页）：顶上分段切 今天 / 近 7 天 / 本月 / 累计，下面这一段的字数与柱状图；
 * 再往下是打字质量（速度、首选率、一次打对、平均码长）、其他数字与学习语言的词汇按级进度。PC 的一年日历与云服务用量不做。
 */
class StatsActivity : PageActivity() {
    override val title = R.string.page_stats

    private var period = Period.TODAY

    private enum class Period(val label: Int) {
        TODAY(R.string.stats_period_today),
        RECENT(R.string.stats_recent),
        MONTH(R.string.stats_month),
        TOTAL(R.string.stats_total),
    }

    override fun content(): List<View> {
        val stats = StatsDetail.load(this) ?: return listOf(ui.group(null, ui.row(R.string.stats_unavailable, null)))
        return listOf(
            ui.segmented(Period.entries.map { getString(it.label) }, period.ordinal) {
                period = Period.entries[it]
                rebuild()
            }.apply {
                layoutParams = LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT).apply { topMargin = ui.dp(16) }
            },
            periodCard(stats),
            ui.group(getString(R.string.stats_quality), listOf(quality(stats)), getString(R.string.stats_details_note)),
            others(stats),
            vocabulary(stats.vocabulary),
        )
    }

    /** 这一段的字数（大字）、英文词数，下面的柱状图：今天按小时，其余按天（近 7 天只画最后 7 根）。 */
    private fun periodCard(stats: StatsDetail): View {
        val count = when (period) {
            Period.TODAY -> stats.today
            Period.RECENT -> stats.recent
            Period.MONTH -> stats.month
            Period.TOTAL -> stats.total
        }
        val days = if (period == Period.RECENT) stats.days.takeLast(7) else stats.days
        val (values, labels) = if (period == Period.TODAY) {
            stats.hours to hourLabels(stats.hour)
        } else {
            days.map { it.hanzi } to BarChart.dayLabels(days.map { it.date })
        }
        return ui.card(
            LinearLayout(this).apply {
                orientation = LinearLayout.VERTICAL
                setPadding(ui.dp(18), ui.dp(16), ui.dp(18), ui.dp(12))
                addView(LinearLayout(context).apply {
                    orientation = LinearLayout.HORIZONTAL
                    gravity = Gravity.BOTTOM
                    addView(ui.text(String.format("%,d", count.hanzi), 40f, ui.colors.text, bold = true))
                    addView(ui.text(getString(R.string.stats_today_unit, count.english), 14f, ui.colors.secondary).apply {
                        setPadding(ui.dp(8), 0, 0, ui.dp(6))
                    })
                })
                addView(BarChart(context, ui.colors, values, labels), LinearLayout.LayoutParams(MATCH_PARENT, ui.dp(104)).apply { topMargin = ui.dp(14) })
                // 累计没有自己的图：柱子仍是最近 30 天，标一句免得以为是全部
                if (period == Period.TOTAL) {
                    addView(ui.text(getString(R.string.stats_days_chart), 12f, ui.colors.secondary).apply { setPadding(0, ui.dp(6), 0, 0) })
                }
            },
        ).apply {
            layoutParams = LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT).apply { topMargin = ui.dp(16) }
        }
    }

    /** 24 根柱子，最后一根是现在这个钟点；每 6 小时标一次。 */
    private fun hourLabels(hour: Int) = (0 until 24)
        .filter { index -> (hour - 23 + index + 24) % 6 == 0 }
        .associateWith { index -> "${(hour - 23 + index + 24) % 24}:00" }

    /** 两行两列：速度、首选率、一次打对、平均码长；速度按选的时段给今天或平均。 */
    private fun quality(stats: StatsDetail): View {
        val none = getString(R.string.stats_none)
        val speed = if (period == Period.TODAY) stats.todaySpeed else stats.averageSpeed
        val rate = { value: Double? -> value?.let { getString(R.string.stats_percent, it * 100) } ?: none }
        val cells = listOf(
            (speed?.let { getString(R.string.stats_speed_value, it) } ?: none) to
                if (period == Period.TODAY) R.string.stats_today_speed else R.string.stats_average_speed,
            rate(stats.firstChoiceRate) to R.string.stats_first_choice,
            rate(stats.cleanRate) to R.string.stats_clean,
            (stats.keysPerHanzi?.let { getString(R.string.stats_code_length_value, it) } ?: none) to R.string.stats_code_length,
        )
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            cells.chunked(2).forEachIndexed { row, pair ->
                if (row > 0) addView(ui.divider(0))
                addView(LinearLayout(context).apply {
                    orientation = LinearLayout.HORIZONTAL
                    pair.forEachIndexed { column, (value, label) ->
                        if (column > 0) addView(View(context).apply { setBackgroundColor(ui.colors.divider) }, LinearLayout.LayoutParams(maxOf(1, ui.dp(1) / 2), MATCH_PARENT))
                        addView(LinearLayout(context).apply {
                            orientation = LinearLayout.VERTICAL
                            setPadding(ui.dp(16), ui.dp(12), ui.dp(16), ui.dp(12))
                            addView(ui.text(value, 19f, ui.colors.text, bold = true))
                            addView(ui.text(getString(label), 12f, ui.colors.secondary).apply { setPadding(0, ui.dp(3), 0, 0) })
                        }, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
                    }
                })
            }
        }
    }

    /** 最多的一天、有输入的天数、有输入的日子平均。 */
    private fun others(stats: StatsDetail): View {
        val none = getString(R.string.stats_none)
        val best = stats.bestDay?.let { "${shortDate(it.date)} · ${hanzi(it.hanzi)}" } ?: none
        return ui.group(
            null,
            valueRow(R.string.stats_best_day, best),
            valueRow(R.string.stats_active_days, getString(R.string.stats_days_value, stats.activeDays)),
            valueRow(R.string.stats_average, if (stats.activeDays == 0L) none else hanzi(stats.average)),
        )
    }

    private fun valueRow(label: Int, value: String) = LinearLayout(this).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        minimumHeight = ui.dp(48)
        setPadding(ui.dp(16), ui.dp(8), ui.dp(16), ui.dp(8))
        addView(ui.text(getString(label), 16f, ui.colors.text), LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
        addView(ui.text(value, 15f, ui.colors.secondary))
    }

    /** 学习语言的词汇：一句汇总，有等级表时一级一行（见过与看熟占这一级的比例）。 */
    private fun vocabulary(vocabulary: StatsDetail.Vocabulary?): View {
        if (vocabulary == null) return ui.group(getString(R.string.stats_vocabulary), listOf(ui.row(R.string.stats_vocabulary_off, null)))
        val name = SettingChoices.learningLanguageName(vocabulary.language)
        val summary = if (vocabulary.seen == 0L) {
            getString(R.string.stats_vocabulary_empty, name)
        } else {
            getString(R.string.stats_vocabulary_line, name, vocabulary.seen, vocabulary.familiar, vocabulary.committed, vocabulary.newThisWeek)
        }
        val rows = mutableListOf<View>(ui.row(summary, null))
        if (vocabulary.levels.isNotEmpty()) {
            rows += LinearLayout(this).apply {
                orientation = LinearLayout.VERTICAL
                setPadding(0, ui.dp(6), 0, ui.dp(8))
                vocabulary.levels.forEach { addView(levelRow(it)) }
            }
        }
        return ui.group(
            getString(R.string.stats_vocabulary), rows,
            if (vocabulary.levels.isNotEmpty()) getString(R.string.stats_levels_note) else null,
        )
    }

    private fun levelRow(level: StatsDetail.Level) = LinearLayout(this).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(ui.dp(16), ui.dp(6), ui.dp(16), ui.dp(6))
        addView(LinearLayout(context).apply {
            orientation = LinearLayout.HORIZONTAL
            addView(ui.text(level.name, 14f, ui.colors.text, bold = true), LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
            addView(ui.text(getString(R.string.stats_level_value, level.seen, level.familiar, level.total), 12f, ui.colors.secondary))
        })
        addView(progress(level), LinearLayout.LayoutParams(MATCH_PARENT, ui.dp(6)).apply { topMargin = ui.dp(6) })
    }

    /** 一根进度条：浅色是见过的，深色是看熟的，底是这一级的全部。 */
    private fun progress(level: StatsDetail.Level) = LinearLayout(this).apply {
        orientation = LinearLayout.HORIZONTAL
        weightSum = 1f
        background = ui.rounded(ui.colors.fill, 3f)
        val total = level.total.coerceAtLeast(1).toFloat()
        val familiar = level.familiar.coerceAtMost(level.total) / total
        val seen = (level.seen.coerceAtMost(level.total) - level.familiar.coerceAtMost(level.seen)) / total
        addView(View(context).apply { background = ui.rounded(ui.colors.brand, 3f) }, LinearLayout.LayoutParams(0, MATCH_PARENT, familiar))
        addView(View(context).apply { background = ui.rounded(ui.colors.brand and 0x00FFFFFF or 0x59000000, 3f) }, LinearLayout.LayoutParams(0, MATCH_PARENT, seen))
        addView(View(context), LinearLayout.LayoutParams(0, MATCH_PARENT, (1f - familiar - seen).coerceAtLeast(0f)))
    }

    private fun hanzi(count: Long) = getString(R.string.stats_hanzi_value, count)

    private fun shortDate(date: String) = date.split('-').let { getString(R.string.stats_date, it[1].toInt(), it[2].toInt()) }
}
