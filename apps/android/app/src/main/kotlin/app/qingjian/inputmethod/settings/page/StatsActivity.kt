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
 * 输入统计详情（参照 PC 设置程序的统计页）：四个时段的字数、过去 24 小时与最近 30 天的柱状图、输入详情、学习语言的词汇按级进度。
 * PC 的一年日历与云服务用量不做。
 */
class StatsActivity : PageActivity() {
    override val title = R.string.page_stats

    override fun content(): List<View> {
        val stats = StatsDetail.load(this) ?: return listOf(ui.card(ui.row(R.string.stats_unavailable, null)))
        return buildList {
            add(periods(stats))
            add(ui.sectionTitle(R.string.stats_hours))
            add(chartCard(stats.hours, hourLabels(stats.hour)))
            add(ui.sectionTitle(R.string.stats_days_chart))
            add(chartCard(stats.days.map { it.hanzi }, BarChart.dayLabels(stats.days.map { it.date })))
            add(ui.sectionTitle(R.string.stats_details))
            add(details(stats))
            add(ui.sectionTitle(R.string.stats_vocabulary))
            add(vocabulary(stats.vocabulary))
        }
    }

    /** 两行两列：今天、近 7 天、本月、累计。 */
    private fun periods(stats: StatsDetail) = LinearLayout(this).apply {
        orientation = LinearLayout.VERTICAL
        listOf(
            listOf(R.string.stats_period_today to stats.today, R.string.stats_recent to stats.recent),
            listOf(R.string.stats_month to stats.month, R.string.stats_total to stats.total),
        ).forEachIndexed { row, pair ->
            addView(LinearLayout(context).apply {
                orientation = LinearLayout.HORIZONTAL
                pair.forEachIndexed { column, (label, count) ->
                    addView(periodCard(label, count), LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f).apply {
                        if (column > 0) marginStart = ui.dp(12)
                    })
                }
            }, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT).apply { if (row > 0) topMargin = ui.dp(12) })
        }
    }

    private fun periodCard(label: Int, count: StatsDetail.Count) = ui.card(
        LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(ui.dp(16), ui.dp(14), ui.dp(16), ui.dp(14))
            addView(ui.text(getString(label), 13f, ui.colors.brand, bold = true))
            addView(ui.text(hanzi(count.hanzi), 22f, ui.colors.text, bold = true).apply { setPadding(0, ui.dp(8), 0, ui.dp(4)) })
            addView(ui.text(getString(R.string.stats_english, count.english), 12f, ui.colors.secondary))
        },
    )

    private fun chartCard(values: List<Long>, labels: Map<Int, String>) = ui.card(
        BarChart(this, ui.colors, values, labels).apply {
            layoutParams = LinearLayout.LayoutParams(MATCH_PARENT, ui.dp(150)).apply {
                setMargins(ui.dp(14), ui.dp(16), ui.dp(14), ui.dp(10))
            }
        },
    )

    /** 24 根柱子，最后一根是现在这个钟点；每 6 小时标一次。 */
    private fun hourLabels(hour: Int) = (0 until 24)
        .filter { index -> (hour - 23 + index + 24) % 6 == 0 }
        .associateWith { index -> "${(hour - 23 + index + 24) % 24}:00" }

    private fun details(stats: StatsDetail): View {
        val none = getString(R.string.stats_none)
        val speed = { value: Double? -> value?.let { getString(R.string.stats_speed_value, it) } ?: none }
        val rate = { value: Double? -> value?.let { getString(R.string.stats_percent, it * 100) } ?: none }
        val best = stats.bestDay?.let { "${shortDate(it.date)} · ${hanzi(it.hanzi)}" } ?: none
        val rows = listOf(
            R.string.stats_best_day to best,
            R.string.stats_active_days to getString(R.string.stats_days_value, stats.activeDays),
            R.string.stats_average to if (stats.activeDays == 0L) none else hanzi(stats.average),
            R.string.stats_today_speed to speed(stats.todaySpeed),
            R.string.stats_average_speed to speed(stats.averageSpeed),
            R.string.stats_first_choice to rate(stats.firstChoiceRate),
            R.string.stats_clean to rate(stats.cleanRate),
            R.string.stats_code_length to (stats.keysPerHanzi?.let { getString(R.string.stats_code_length_value, it) } ?: none),
        )
        val views = mutableListOf<View>()
        rows.forEachIndexed { index, (label, value) ->
            if (index > 0) views += ui.divider()
            views += valueRow(getString(label), value)
        }
        views += ui.note(R.string.stats_details_note)
        return ui.card(*views.toTypedArray())
    }

    private fun valueRow(label: String, value: String) = LinearLayout(this).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        setPadding(ui.dp(18), ui.dp(14), ui.dp(18), ui.dp(14))
        addView(ui.text(label, 15f, ui.colors.text), LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
        addView(ui.text(value, 14f, ui.colors.secondary))
    }

    /** 学习语言的词汇：一句汇总，有等级表时一级一行（见过与看熟占这一级的比例）。 */
    private fun vocabulary(vocabulary: StatsDetail.Vocabulary?): View {
        if (vocabulary == null) return ui.card(ui.row(R.string.stats_vocabulary_off, null))
        val name = SettingChoices.learningLanguageName(vocabulary.language)
        val summary = if (vocabulary.seen == 0L) {
            getString(R.string.stats_vocabulary_empty, name)
        } else {
            getString(R.string.stats_vocabulary_line, name, vocabulary.seen, vocabulary.familiar, vocabulary.committed, vocabulary.newThisWeek)
        }
        val views = mutableListOf<View>(
            ui.text(summary, 14f, ui.colors.text).apply {
                setPadding(ui.dp(18), ui.dp(16), ui.dp(18), ui.dp(12))
                setLineSpacing(0f, 1.25f)
            },
        )
        vocabulary.levels.forEach { views += levelRow(it) }
        if (vocabulary.levels.isNotEmpty()) views += ui.note(R.string.stats_levels_note)
        return ui.card(*views.toTypedArray())
    }

    private fun levelRow(level: StatsDetail.Level) = LinearLayout(this).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(ui.dp(18), ui.dp(6), ui.dp(18), ui.dp(10))
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
        background = ui.rounded(ui.colors.divider, 3f)
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
