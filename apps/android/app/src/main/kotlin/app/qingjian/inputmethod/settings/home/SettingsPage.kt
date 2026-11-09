package app.qingjian.inputmethod.settings.home

import android.app.Activity
import android.app.AlertDialog
import android.content.Intent
import android.view.Gravity
import android.view.View
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.LinearLayout
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.clipboard.sync.ClipboardSync
import app.qingjian.inputmethod.cloud.CloudConfig
import app.qingjian.inputmethod.core.store.SettingChoices
import app.qingjian.inputmethod.core.store.Themes
import app.qingjian.inputmethod.panel.base.PanelIcons.Icon
import app.qingjian.inputmethod.settings.Settings
import app.qingjian.inputmethod.settings.page.AboutActivity
import app.qingjian.inputmethod.settings.page.AdvancedActivity
import app.qingjian.inputmethod.settings.page.AppearanceActivity
import app.qingjian.inputmethod.settings.page.ClipboardSettingsActivity
import app.qingjian.inputmethod.settings.page.ClipboardSyncActivity
import app.qingjian.inputmethod.settings.page.CloudSettingsActivity
import app.qingjian.inputmethod.settings.page.DictionariesActivity
import app.qingjian.inputmethod.settings.page.InputSettingsActivity
import app.qingjian.inputmethod.settings.page.KeyboardSettingsActivity
import app.qingjian.inputmethod.settings.page.VoiceSettingsActivity
import app.qingjian.inputmethod.settings.widget.SettingsWidgets
import app.qingjian.inputmethod.voice.VoiceConfig

/**
 * 设置 tab：顶上是「青简是不是当前输入法」，下面按 打字 / 学语言 / 剪贴板 / 联网服务 / 数据与关于 分组，
 * 每行右边是当前值，点进对应的二级页。
 */
class SettingsPage(private val activity: Activity, private val ui: SettingsWidgets) : AppPage {
    private val colors get() = ui.colors

    /** 页面上画着的值；从二级页回来变了才重建。 */
    private var shown: List<Any?> = emptyList()

    private fun signature(settings: Settings) = listOf(
        ImeState.of(activity), settings.layout, settings.theme, settings.appearance, settings.inputOptions.learningLanguage,
        settings.recordClipboard, settings.clipboardLimit, ClipboardSync.enabled(activity),
        CloudConfig.load(activity).ready, VoiceConfig.load(activity).provider,
    )

    override fun stale() = signature(Settings.load(activity)) != shown

    override fun header(): View = ui.pageTitle(R.string.tab_settings)

    override fun build(): View {
        val settings = Settings.load(activity)
        shown = signature(settings)
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            addView(statusCard(ImeState.of(activity), settings), LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT).apply { topMargin = ui.dp(20) })
            addView(ui.group(
                R.string.section_typing,
                ui.entry(Icon.PEN, R.string.page_input, activity.getString(layoutName(settings.layout))) { open(InputSettingsActivity::class.java) },
                ui.entry(Icon.KEYBOARD, R.string.page_keyboard, activity.getString(R.string.keyboard_summary)) { open(KeyboardSettingsActivity::class.java) },
                ui.entry(Icon.PALETTE, R.string.page_appearance, "${Themes.name(settings.theme)} · ${activity.getString(settings.appearance.label)}") {
                    open(AppearanceActivity::class.java)
                },
                ui.entry(Icon.BOOK, R.string.page_dictionaries, null) { open(DictionariesActivity::class.java) },
            ))
            addView(learning(settings))
            addView(ui.group(
                R.string.section_clipboard,
                ui.entry(
                    Icon.CLIPBOARD, R.string.section_clipboard,
                    if (settings.recordClipboard) activity.getString(R.string.clipboard_limit_value, settings.clipboardLimit) else activity.getString(R.string.value_off),
                ) { open(ClipboardSettingsActivity::class.java) },
                ui.entry(Icon.GLOBE, R.string.page_clipboard_sync, activity.getString(if (ClipboardSync.enabled(activity)) R.string.value_on else R.string.value_off)) {
                    open(ClipboardSyncActivity::class.java)
                },
            ))
            addView(ui.group(
                R.string.section_online,
                ui.entry(Icon.CLOUD, R.string.section_cloud, activity.getString(if (CloudConfig.load(activity).ready) R.string.value_set else R.string.value_unset)) {
                    open(CloudSettingsActivity::class.java)
                },
                ui.entry(Icon.MIC, R.string.page_voice, activity.getString(if (VoiceConfig.load(activity).provider == VoiceConfig.PROVIDER_NONE) R.string.value_unset else R.string.value_set)) {
                    open(VoiceSettingsActivity::class.java)
                },
                footer = R.string.online_note,
            ))
            addView(ui.group(
                null,
                ui.entry(Icon.LOCK, R.string.page_advanced, null, muted = true) { open(AdvancedActivity::class.java) },
                ui.entry(Icon.INFO, R.string.page_about, null, muted = true) { open(AboutActivity::class.java) },
            ))
        }
    }

    /** 当前输入法：是青简就显示在用的方案、旁边「切换」；还没启用就去系统设置。 */
    private fun statusCard(state: ImeState, settings: Settings) = ui.card(
        LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(ui.dp(16), ui.dp(12), ui.dp(12), ui.dp(12))
            addView(ui.tile(Icon.KEYBOARD, 40))
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.VERTICAL
                setPadding(ui.dp(12), 0, ui.dp(8), 0)
                val title = when {
                    state.ready -> R.string.status_current
                    state.enabled -> R.string.setup_pick
                    else -> R.string.setup_enable
                }
                addView(ui.text(activity.getString(title), 16f, colors.text, bold = true))
                addView(ui.text(
                    if (state.ready) activity.getString(layoutName(settings.layout)) else activity.getString(if (state.enabled) R.string.setup_pick_note else R.string.setup_enable_note),
                    13f, colors.secondary,
                ).apply { setPadding(0, ui.dp(3), 0, 0) })
            }, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
            addView(ui.text(activity.getString(if (state.enabled) R.string.setup_switch else R.string.setup_go), 14f, colors.brand, bold = true).apply {
                background = ui.rounded(colors.fill, 16f)
                setPadding(ui.dp(14), ui.dp(7), ui.dp(14), ui.dp(7))
                setOnClickListener { if (state.enabled) ImeState.showPicker(activity) else ImeState.openSystemSettings(activity) }
            })
        },
    )

    /** 学习语言：点开直接弹单选框，换了键盘立即换译词，生词本与统计页也跟着换。 */
    private fun learning(settings: Settings): View {
        val languages = SettingChoices.learningLanguages
        val current = settings.inputOptions.learningLanguage
        return ui.group(
            R.string.section_learning_language,
            ui.entry(Icon.TRANSLATE, R.string.learning_language_title, SettingChoices.learningLanguageName(current)) {
                AlertDialog.Builder(activity)
                    .setTitle(R.string.learning_language_title)
                    .setSingleChoiceItems(languages.map { it.label }.toTypedArray(), languages.indexOfFirst { it.key == current }) { dialog, which ->
                        dialog.dismiss()
                        Settings.preferences(activity).edit().putString(Settings.KEY_LEARNING_LANGUAGE, languages[which].key).apply()
                    }
                    .setOnDismissListener { if (stale()) activity.recreate() }
                    .show()
            },
            footer = R.string.learning_language_note,
        )
    }

    private fun open(page: Class<*>) = activity.startActivity(Intent(activity, page))

    companion object {
        fun layoutName(layout: Settings.Layout) = when (layout) {
            Settings.Layout.QWERTY -> R.string.layout_qwerty_short
            Settings.Layout.NINE_KEY -> R.string.layout_nine_short
            Settings.Layout.SHUANGPIN -> R.string.layout_shuangpin
            Settings.Layout.WUBI -> R.string.layout_wubi_short
            Settings.Layout.ZHUYIN -> R.string.layout_zhuyin_short
        }
    }
}
