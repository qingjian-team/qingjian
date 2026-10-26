package app.qingjian.inputmethod.settings.home

import android.app.Activity
import android.content.Intent
import android.view.Gravity
import android.view.View
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.ImageView
import android.widget.LinearLayout
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.core.store.UsageSummary
import app.qingjian.inputmethod.core.store.Vocabulary
import app.qingjian.inputmethod.panel.base.PanelIcons.Icon
import app.qingjian.inputmethod.settings.page.AboutActivity
import app.qingjian.inputmethod.settings.page.AdvancedActivity
import app.qingjian.inputmethod.settings.page.AppearanceActivity
import app.qingjian.inputmethod.settings.page.DictionariesActivity
import app.qingjian.inputmethod.settings.page.KeyboardSettingsActivity
import app.qingjian.inputmethod.settings.page.StatsActivity
import app.qingjian.inputmethod.settings.page.VoiceSettingsActivity
import app.qingjian.inputmethod.settings.widget.SettingsWidgets
import app.qingjian.inputmethod.theme.AppColors

/** 我的 tab：品牌头像行、累计数字卡（点开统计详情），下面是分组的设置入口；输入统计的入口在首页看板。 */
class MePage(private val activity: Activity, private val ui: SettingsWidgets) : AppPage {
    private val colors get() = ui.colors

    override fun header(): View = ui.pageTitle(R.string.tab_me)

    override fun build(): View {
        val usage = UsageSummary.load(activity)
        val vocabulary = Vocabulary.load(activity)
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            addView(profile(usage))
            addView(totals(usage, vocabulary))
            addView(ui.sectionTitle(R.string.section_settings))
            addView(ui.card(
                ui.entry(Icon.KEYBOARD, AppColors.JADE, R.string.page_keyboard, activity.getString(R.string.keyboard_summary)) {
                    open(KeyboardSettingsActivity::class.java)
                },
                ui.divider(),
                ui.entry(Icon.PALETTE, AppColors.ORANGE, R.string.page_appearance, activity.getString(R.string.appearance_summary)) {
                    open(AppearanceActivity::class.java)
                },
                ui.divider(),
                ui.entry(Icon.MIC, AppColors.PURPLE, R.string.page_voice, activity.getString(R.string.voice_summary)) {
                    open(VoiceSettingsActivity::class.java)
                },
                ui.divider(),
                ui.entry(Icon.BOOK, AppColors.CYAN, R.string.page_dictionaries, activity.getString(R.string.dictionaries_summary)) {
                    open(DictionariesActivity::class.java)
                },
                ui.divider(),
                ui.entry(Icon.GLOBE, AppColors.BLUE, R.string.shortcut_switch, activity.getString(R.string.switch_summary)) {
                    ImeState.showPicker(activity)
                },
            ))
            addView(ui.card(
                ui.entry(Icon.SETTINGS, AppColors.PINK, R.string.page_advanced, activity.getString(R.string.advanced_summary)) {
                    open(AdvancedActivity::class.java)
                },
                ui.divider(),
                ui.entry(Icon.INFO, AppColors.PURPLE, R.string.page_about, activity.getString(R.string.about_summary)) {
                    open(AboutActivity::class.java)
                },
            ), LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, WRAP_CONTENT).apply { topMargin = ui.dp(12) })
        }
    }

    private fun profile(usage: UsageSummary) = LinearLayout(activity).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        setPadding(ui.dp(4), ui.dp(6), 0, ui.dp(20))
        addView(ImageView(activity).apply { setImageResource(R.drawable.ic_brand) }, LinearLayout.LayoutParams(ui.dp(58), ui.dp(58)))
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(ui.dp(14), 0, 0, 0)
            addView(ui.text(activity.getString(R.string.me_title), 22f, colors.text, bold = true))
            addView(ui.text(activity.getString(R.string.me_note, usage.days), 12.5f, colors.secondary).apply { setPadding(0, ui.dp(6), 0, 0) })
        })
    }

    private fun totals(usage: UsageSummary, vocabulary: Vocabulary) = ui.card(
        LinearLayout(activity).apply {
            setOnClickListener { open(StatsActivity::class.java) }
            orientation = LinearLayout.HORIZONTAL
            setPadding(0, ui.dp(20), 0, ui.dp(18))
            listOf(
                R.string.me_total to usage.totalHanzi,
                R.string.me_words to vocabulary.seen,
                R.string.me_days to usage.days.toLong(),
            ).forEach { (label, value) ->
                addView(LinearLayout(activity).apply {
                    orientation = LinearLayout.VERTICAL
                    gravity = Gravity.CENTER_HORIZONTAL
                    addView(ui.text(value.toString(), 24f, colors.text, bold = true).apply { gravity = Gravity.CENTER })
                    addView(ui.text(activity.getString(label), 12f, colors.secondary).apply {
                        gravity = Gravity.CENTER
                        setPadding(0, ui.dp(8), 0, 0)
                    })
                }, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
            }
        },
    )

    private fun open(page: Class<*>) = activity.startActivity(Intent(activity, page))
}
