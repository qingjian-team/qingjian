package app.qingjian.inputmethod.settings.page

import android.content.Intent
import android.view.View
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.core.store.SettingChoices
import app.qingjian.inputmethod.settings.Settings

/** 剪贴板二级页：记不记录、最多存多少条、代码高亮，底下是局域网同步的入口。 */
class ClipboardSettingsActivity : PageActivity() {
    override val title = R.string.section_clipboard

    override fun content(): List<View> {
        val settings = Settings.load(this)
        val rows = mutableListOf<View>(
            ui.toggle(R.string.record_clipboard_title, null, settings.recordClipboard) {
                prefs.edit().putBoolean(Settings.KEY_RECORD_CLIPBOARD, it).apply()
                rebuild()
            },
        )
        if (settings.recordClipboard) {
            rows += limitPicker(settings)
            rows += ui.toggle(R.string.code_highlight_title, R.string.code_highlight_note, settings.codeHighlight) {
                prefs.edit().putBoolean(Settings.KEY_CODE_HIGHLIGHT, it).apply()
            }
        }
        return listOf(
            ui.group(null, rows, getString(R.string.record_clipboard_note)),
            ui.group(
                null,
                ui.link(R.string.page_clipboard_sync, null) { startActivity(Intent(this, ClipboardSyncActivity::class.java)) },
            ),
        )
    }

    /** 最多存多少条；改小了下次记的时候丢最旧的。 */
    private fun limitPicker(settings: Settings): View {
        val limits = SettingChoices.clipboardLimits
        val selected = limits.indexOf(settings.clipboardLimit).coerceAtLeast(0)
        return ui.picker(
            R.string.clipboard_limit, getString(R.string.clipboard_limit_value, limits[selected]),
            limits.map { getString(R.string.clipboard_limit_value, it) }, selected,
        ) {
            prefs.edit().putInt(Settings.KEY_CLIPBOARD_LIMIT, limits[it]).apply()
            rebuild()
        }
    }
}
