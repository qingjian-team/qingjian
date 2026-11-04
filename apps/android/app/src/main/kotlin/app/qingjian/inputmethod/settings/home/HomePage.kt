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
import app.qingjian.inputmethod.core.store.Vocabulary
import app.qingjian.inputmethod.core.store.VocabularyWord
import app.qingjian.inputmethod.panel.base.PanelIcons.Icon
import app.qingjian.inputmethod.settings.page.StatsActivity
import app.qingjian.inputmethod.settings.widget.BarChart
import app.qingjian.inputmethod.settings.widget.SettingsWidgets

/**
 * 首页 tab：没切到青简时是标语与两步引导；切好了是今日统计卡（点开 [StatsActivity]）与最近见过的生词。最下面一直有试打框。
 * 功能入口都在设置 tab，这里不重复放。
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

    override fun header(): View = ui.pageTitle(R.string.app_name)

    override fun build(): View {
        val state = state().also { shown = it }
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            if (state.first.ready) {
                addView(spaced(statsCard(StatsDetail.load(activity, vocabulary = false))))
                recentWords()?.let(::addView)
            } else {
                addView(spaced(welcome()))
                addView(setupSteps(state.first))
            }
            addView(ui.group(activity.getString(R.string.section_try), listOf(tryOutField())))
        }
    }

    private fun spaced(view: View) = view.apply {
        layoutParams = LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT).apply { topMargin = ui.dp(20) }
    }

    /** 还没用上青简时的头图：图标、标语、一句介绍。 */
    private fun welcome() = ui.card(
        LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER_HORIZONTAL
            setPadding(ui.dp(20), ui.dp(28), ui.dp(20), ui.dp(24))
            addView(ImageView(activity).apply { setImageResource(R.drawable.ic_brand) }, LinearLayout.LayoutParams(ui.dp(64), ui.dp(64)))
            addView(ui.text(activity.getString(R.string.tagline), 19f, colors.text, bold = true).apply {
                gravity = Gravity.CENTER
                setPadding(0, ui.dp(16), 0, ui.dp(8))
            })
            addView(ui.text(activity.getString(R.string.welcome_note), 14f, colors.secondary).apply {
                gravity = Gravity.CENTER
                setLineSpacing(0f, 1.3f)
            })
        },
    )

    /** 两步：在系统里启用、切换成青简；做完的打勾，没做的给按钮。 */
    private fun setupSteps(state: ImeState) = ui.group(
        activity.getString(R.string.section_setup),
        listOf(
            step(1, R.string.setup_enable, null, done = state.enabled, button = R.string.setup_go) { ImeState.openSystemSettings(activity) },
            step(2, R.string.setup_pick_step, R.string.setup_pick_note, done = state.current, button = R.string.setup_switch, enabled = state.enabled) {
                ImeState.showPicker(activity)
            },
        ),
    )

    private fun step(number: Int, title: Int, note: Int?, done: Boolean, button: Int, enabled: Boolean = true, onClick: () -> Unit) =
        LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            minimumHeight = ui.dp(64)
            setPadding(ui.dp(16), ui.dp(10), ui.dp(16), ui.dp(10))
            addView(
                if (done) {
                    ui.icon(Icon.CHECK, colors.onBrand, 16f).apply { background = ui.rounded(colors.brand, 14f) }
                } else {
                    ui.text(number.toString(), 14f, colors.brand, bold = true).apply {
                        gravity = Gravity.CENTER
                        background = ui.rounded(colors.brandSoft, 14f)
                    }
                },
                LinearLayout.LayoutParams(ui.dp(28), ui.dp(28)),
            )
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.VERTICAL
                setPadding(ui.dp(12), 0, ui.dp(8), 0)
                addView(ui.text(activity.getString(title), 16f, if (done) colors.secondary else colors.text))
                if (!done) note?.let { addView(ui.text(activity.getString(it), 13f, colors.secondary).apply { setPadding(0, ui.dp(3), 0, 0) }) }
            }, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
            if (done) {
                addView(ui.text(activity.getString(R.string.setup_done), 14f, colors.secondary))
            } else if (enabled) {
                addView(ui.text(activity.getString(button), 14f, colors.onBrand, bold = true).apply {
                    background = ui.rounded(colors.brand, 17f)
                    setPadding(ui.dp(16), ui.dp(8), ui.dp(16), ui.dp(8))
                    setOnClickListener { onClick() }
                })
            }
        }

    /** 今日字数（大字）、最近 30 天柱状图、近 7 天 / 首选率 / 每字键数；点整张卡进统计详情。 */
    private fun statsCard(stats: StatsDetail?): View {
        if (stats == null) return ui.card(ui.row(R.string.stats_unavailable, null))
        val none = activity.getString(R.string.stats_none)
        return ui.card(
            LinearLayout(activity).apply {
                orientation = LinearLayout.VERTICAL
                background = ui.pressable()
                setPadding(ui.dp(18), ui.dp(16), ui.dp(18), ui.dp(14))
                setOnClickListener { open(StatsActivity::class.java) }
                addView(LinearLayout(activity).apply {
                    orientation = LinearLayout.HORIZONTAL
                    gravity = Gravity.CENTER_VERTICAL
                    addView(ui.text(activity.getString(R.string.stats_today), 13f, colors.secondary), LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
                    addView(ui.icon(Icon.CHEVRON, colors.faint, 18f))
                })
                addView(LinearLayout(activity).apply {
                    orientation = LinearLayout.HORIZONTAL
                    gravity = Gravity.BOTTOM
                    setPadding(0, ui.dp(6), 0, 0)
                    addView(ui.text(number(stats.today.hanzi), 40f, colors.text, bold = true))
                    addView(ui.text(activity.getString(R.string.stats_today_unit, stats.today.english), 14f, colors.secondary).apply {
                        setPadding(ui.dp(8), 0, 0, ui.dp(6))
                    })
                })
                addView(
                    BarChart(activity, colors, stats.days.map { it.hanzi }, emptyMap()),
                    LinearLayout.LayoutParams(MATCH_PARENT, ui.dp(52)).apply { topMargin = ui.dp(12) },
                )
                addView(ui.divider(0).apply { (layoutParams as LinearLayout.LayoutParams).topMargin = ui.dp(12) })
                addView(LinearLayout(activity).apply {
                    orientation = LinearLayout.HORIZONTAL
                    setPadding(0, ui.dp(12), 0, 0)
                    listOf(
                        number(stats.recent.hanzi) to R.string.stats_recent,
                        (stats.firstChoiceRate?.let { activity.getString(R.string.stats_percent, it * 100) } ?: none) to R.string.stats_first_choice,
                        (stats.keysPerHanzi?.let { String.format("%.1f", it) } ?: none) to R.string.stats_keys_per_hanzi,
                    ).forEach { (value, label) ->
                        addView(LinearLayout(activity).apply {
                            orientation = LinearLayout.VERTICAL
                            addView(ui.text(value, 17f, colors.text, bold = true))
                            addView(ui.text(activity.getString(label), 12f, colors.secondary).apply { setPadding(0, ui.dp(3), 0, 0) })
                        }, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
                    }
                })
            },
        )
    }

    /** 最近见过的几个生词，标题右边进生词本；没开学习语言或还没有词就不显示。 */
    private fun recentWords(): View? {
        val vocabulary = Vocabulary.load(activity, Vocabulary.Query(Vocabulary.Filter.WEEK))
        if (vocabulary.off || vocabulary.words.isEmpty()) return null
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(0, ui.dp(24), 0, 0)
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.BOTTOM
                addView(ui.label(activity.getString(R.string.home_recent_words)), LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
                addView(ui.text(activity.getString(R.string.tab_vocabulary), 13f, colors.brand).apply {
                    setPadding(ui.dp(16), 0, ui.dp(8), ui.dp(7))
                    setOnClickListener { openTab(MainActivity.TAB_VOCABULARY) }
                })
            })
            val card = ui.card()
            vocabulary.words.take(RECENT_WORDS).forEachIndexed { index, word ->
                if (index > 0) card.addView(ui.divider())
                card.addView(wordRow(word))
            }
            addView(card)
        }
    }

    private fun wordRow(word: VocabularyWord) = LinearLayout(activity).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        minimumHeight = ui.dp(56)
        setPadding(ui.dp(16), ui.dp(8), ui.dp(16), ui.dp(8))
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.BOTTOM
                addView(ui.text(word.word, 17f, colors.text, bold = true))
                addView(ui.text(word.source, 14f, colors.secondary).apply { setPadding(ui.dp(8), 0, 0, ui.dp(1)) })
            })
            addView(ui.text(activity.getString(R.string.vocabulary_seen_times, word.seen), 12f, colors.secondary).apply {
                setPadding(0, ui.dp(3), 0, 0)
            })
        }, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
        val (label, color) = when {
            word.known -> R.string.vocabulary_known to colors.brand
            word.fresh -> R.string.vocabulary_fresh to colors.fresh
            else -> R.string.vocabulary_familiar to colors.secondary
        }
        addView(ui.text(activity.getString(label), 12f, color).apply {
            background = ui.rounded(color and 0x00FFFFFF or 0x24000000, 10f)
            setPadding(ui.dp(8), ui.dp(3), ui.dp(8), ui.dp(3))
        })
    }

    /** 千位分隔：12,345。 */
    private fun number(value: Long) = String.format("%,d", value)

    private fun tryOutField() = EditText(activity).apply {
        setHint(R.string.try_hint)
        setHintTextColor(colors.faint)
        setTextColor(colors.text)
        textSize = 16f
        background = null
        minLines = 2
        gravity = Gravity.TOP
        setPadding(ui.dp(16), ui.dp(14), ui.dp(16), ui.dp(14))
    }

    private fun open(page: Class<*>) = activity.startActivity(Intent(activity, page))

    companion object {
        private const val RECENT_WORDS = 3
    }
}
