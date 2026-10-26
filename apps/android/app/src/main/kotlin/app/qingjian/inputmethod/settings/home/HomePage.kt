package app.qingjian.inputmethod.settings.home

import android.app.Activity
import android.content.Intent
import android.view.Gravity
import android.view.View
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.EditText
import android.widget.ImageView
import android.widget.LinearLayout
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.core.store.StatsDetail
import app.qingjian.inputmethod.core.store.UsageSummary
import app.qingjian.inputmethod.panel.base.PanelIcons.Icon
import app.qingjian.inputmethod.settings.page.AboutActivity
import app.qingjian.inputmethod.settings.page.KeyboardSettingsActivity
import app.qingjian.inputmethod.settings.page.StatsActivity
import app.qingjian.inputmethod.settings.widget.BarChart
import app.qingjian.inputmethod.settings.widget.SettingsWidgets
import app.qingjian.inputmethod.theme.AppColors

/**
 * 首页 tab：品牌顶栏、没切到青简时的提示卡、输入统计看板（今天 / 近 7 天 / 累计、最近 30 天、四个比例）、常用功能宫格、试打框。
 * 完整的统计在 [StatsActivity]，点看板任一张卡进去。
 */
class HomePage(
    private val activity: Activity,
    private val ui: SettingsWidgets,
    private val openTab: (Int) -> Unit,
) : AppPage {
    private val colors get() = ui.colors

    /** 页面上画着的状态；变了才重建，否则试打框的焦点会被冲掉。 */
    private var shown: Pair<ImeState, Long>? = null

    private fun state() = ImeState.of(activity) to UsageSummary.load(activity).todayHanzi

    override fun stale() = state() != shown

    override fun build(): View {
        val state = state().also { shown = it }
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            if (!state.first.ready) addView(setupCard(state.first), spaced(4))
            dashboard(StatsDetail.load(activity, vocabulary = false)).forEachIndexed { index, card ->
                addView(card, spaced(if (index > 0) 12 else if (state.first.ready) 4 else 14))
            }
            addView(ui.sectionTitle(R.string.section_shortcuts))
            addView(shortcuts())
            addView(ui.sectionTitle(R.string.section_try))
            addView(tryOutField())
        }
    }

    private fun spaced(topDp: Int) = LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT).apply { topMargin = ui.dp(topDp) }

    override fun header(): View = LinearLayout(activity).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        setPadding(ui.dp(4), ui.dp(8), 0, ui.dp(10))
        addView(ImageView(activity).apply { setImageResource(R.drawable.ic_brand) }, LinearLayout.LayoutParams(ui.dp(36), ui.dp(36)))
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(ui.dp(12), 0, 0, 0)
            addView(ui.text(activity.getString(R.string.app_name), 20f, colors.text, bold = true))
            addView(ui.text(activity.getString(R.string.tagline), 12f, colors.secondary).apply { setPadding(0, ui.dp(3), 0, 0) })
        }, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
    }

    /** 默认输入法不是青简才有：没启用先去系统设置，启用了直接弹选择框。 */
    private fun setupCard(state: ImeState) = LinearLayout(activity).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        background = ui.rounded(colors.surface, SettingsWidgets.CARD_RADIUS)
        setPadding(ui.dp(16), ui.dp(16), ui.dp(14), ui.dp(16))
        addView(ui.tile(Icon.KEYBOARD, AppColors.JADE, 40))
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(ui.dp(12), 0, ui.dp(8), 0)
            addView(ui.text(activity.getString(if (state.enabled) R.string.setup_pick else R.string.setup_enable), 15.5f, colors.text, bold = true))
            addView(ui.text(activity.getString(if (state.enabled) R.string.setup_pick_note else R.string.setup_enable_note), 12f, colors.secondary).apply {
                setPadding(0, ui.dp(5), 0, 0)
            })
        }, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
        addView(ui.text(activity.getString(if (state.enabled) R.string.setup_switch else R.string.setup_go), 14f, colors.onBrand, bold = true).apply {
            background = ui.rounded(colors.brand, 18f)
            setPadding(ui.dp(18), ui.dp(9), ui.dp(18), ui.dp(9))
        })
        setOnClickListener {
            if (state.enabled) ImeState.showPicker(activity) else ImeState.openSystemSettings(activity)
        }
    }

    /** 首页统计看板：大数字卡、最近 30 天柱状图、四个比例；点哪张都进统计详情。 */
    private fun dashboard(stats: StatsDetail?): List<View> {
        if (stats == null) return listOf(ui.card(ui.row(R.string.stats_unavailable, null)))
        return listOf(figuresCard(stats), trendCard(stats), ratesCard(stats))
    }

    /** 左边今天（大字，下面英文词数与今天速度），右边上下两格近 7 天与累计。 */
    private fun figuresCard(stats: StatsDetail) = ui.card(
        LinearLayout(activity).apply {
            setOnClickListener { open(StatsActivity::class.java) }
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(ui.dp(20), ui.dp(20), ui.dp(18), ui.dp(18))
            val note = stats.todaySpeed?.let { activity.getString(R.string.stats_today_speed_note, stats.today.english, it) }
                ?: activity.getString(R.string.stats_english, stats.today.english)
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.VERTICAL
                addView(ui.text(activity.getString(R.string.stats_today) + " ›", 13.5f, colors.brand, bold = true))
                addView(ui.text(number(stats.today.hanzi), 36f, colors.text, bold = true).apply { setPadding(0, ui.dp(6), 0, ui.dp(4)) })
                addView(ui.text(note, 12f, colors.secondary))
            }, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1.25f))
            addView(View(activity).apply { setBackgroundColor(colors.divider) }, LinearLayout.LayoutParams(maxOf(1, ui.dp(1)), ui.dp(76)).apply {
                setMargins(ui.dp(8), 0, ui.dp(18), 0)
            })
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.VERTICAL
                addView(smallFigure(R.string.stats_recent, stats.recent.hanzi))
                addView(smallFigure(R.string.stats_total, stats.total.hanzi).apply { setPadding(0, ui.dp(12), 0, 0) })
            }, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
        },
    )

    private fun smallFigure(label: Int, value: Long) = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        addView(ui.text(activity.getString(label), 12.5f, colors.brand, bold = true))
        addView(ui.text(number(value), 19f, colors.text, bold = true).apply { setPadding(0, ui.dp(3), 0, 0) })
    }

    /** 最近 30 天每天的字数，右上角「全部统计 ›」。 */
    private fun trendCard(stats: StatsDetail) = ui.card(
        LinearLayout(activity).apply {
            setOnClickListener { open(StatsActivity::class.java) }
            orientation = LinearLayout.VERTICAL
            setPadding(ui.dp(18), ui.dp(16), ui.dp(16), ui.dp(12))
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                addView(ui.text(activity.getString(R.string.stats_days_chart), 13.5f, colors.brand, bold = true), LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
                addView(ui.text(activity.getString(R.string.stats_more) + " ›", 12.5f, colors.secondary))
            })
            addView(
                BarChart(activity, colors, stats.days.map { it.hanzi }, BarChart.dayLabels(stats.days.map { it.date })),
                LinearLayout.LayoutParams(MATCH_PARENT, ui.dp(104)).apply { topMargin = ui.dp(12) },
            )
        },
    )

    /** 四格：首选率、一次打对、每字键数、有输入的天数；打字太少的显示「—」。 */
    private fun ratesCard(stats: StatsDetail) = ui.card(
        LinearLayout(activity).apply {
            setOnClickListener { open(StatsActivity::class.java) }
            orientation = LinearLayout.HORIZONTAL
            setPadding(ui.dp(4), ui.dp(16), ui.dp(4), ui.dp(14))
            val none = activity.getString(R.string.stats_none)
            val rate = { value: Double? -> value?.let { activity.getString(R.string.stats_percent, it * 100) } ?: none }
            listOf(
                rate(stats.firstChoiceRate) to R.string.stats_first_choice,
                rate(stats.cleanRate) to R.string.stats_clean,
                (stats.keysPerHanzi?.let { String.format("%.1f", it) } ?: none) to R.string.stats_keys_per_hanzi,
                stats.activeDays.toString() to R.string.stats_active_days,
            ).forEach { (value, label) ->
                addView(LinearLayout(activity).apply {
                    orientation = LinearLayout.VERTICAL
                    gravity = Gravity.CENTER_HORIZONTAL
                    addView(ui.text(value, 18f, colors.text, bold = true).apply { gravity = Gravity.CENTER })
                    addView(ui.text(activity.getString(label), 11.5f, colors.secondary).apply {
                        gravity = Gravity.CENTER
                        setPadding(0, ui.dp(6), 0, 0)
                    })
                }, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
            }
        },
    )

    /** 千位分隔：12,345。 */
    private fun number(value: Long) = String.format("%,d", value)

    private fun shortcuts() = ui.card(
        LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            setPadding(ui.dp(4), ui.dp(2), ui.dp(4), ui.dp(2))
            listOf(
                ui.gridItem(Icon.KEYBOARD, AppColors.JADE, R.string.page_keyboard) { open(KeyboardSettingsActivity::class.java) },
                ui.gridItem(Icon.BOOK, AppColors.ORANGE, R.string.tab_vocabulary) { openTab(MainActivity.TAB_VOCABULARY) },
                ui.gridItem(Icon.GLOBE, AppColors.BLUE, R.string.shortcut_switch) { ImeState.showPicker(activity) },
                ui.gridItem(Icon.INFO, AppColors.PURPLE, R.string.page_about) { open(AboutActivity::class.java) },
            ).forEach { addView(it, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f)) }
        },
    )

    private fun tryOutField() = EditText(activity).apply {
        setHint(R.string.try_hint)
        setHintTextColor(colors.faint)
        setTextColor(colors.text)
        textSize = 16f
        background = ui.rounded(colors.surface, SettingsWidgets.CARD_RADIUS)
        minLines = 3
        gravity = Gravity.TOP
        setPadding(ui.dp(18), ui.dp(16), ui.dp(18), ui.dp(16))
    }

    private fun open(page: Class<*>) = activity.startActivity(Intent(activity, page))
}
