package app.qingjian.inputmethod.settings.page

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Bundle
import android.view.View
import android.widget.Toast
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.core.store.SettingChoices
import app.qingjian.inputmethod.settings.Settings
import app.qingjian.inputmethod.verification.Sources
import app.qingjian.inputmethod.verification.Inbox

/** 剪贴板二级页：记不记录、最多存多少条、代码高亮，底下是局域网同步的入口。 */
class ClipboardSettingsActivity : PageActivity() {
    override val title = R.string.section_clipboard
    private var awaitingSms = false
    private var awaitingNotifications = false

    override fun onCreate(savedInstanceState: Bundle?) {
        awaitingSms = savedInstanceState?.getBoolean("awaitingSms") == true
        awaitingNotifications = savedInstanceState?.getBoolean("awaitingNotifications") == true
        super.onCreate(savedInstanceState)
    }

    override fun onSaveInstanceState(outState: Bundle) {
        outState.putBoolean("awaitingSms", awaitingSms)
        outState.putBoolean("awaitingNotifications", awaitingNotifications)
        super.onSaveInstanceState(outState)
    }

    override fun onResume() {
        super.onResume()
        if (awaitingNotifications) {
            awaitingNotifications = false
            prefs.edit().putBoolean(Sources.NOTIFICATIONS, Sources.notificationAccess(this)).apply()
        }
        if (!Sources.sms(this)) Inbox.clear("sms")
        if (!Sources.notifications(this)) Inbox.clear("notification")
        rebuild()
    }

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
                R.string.verification_section,
                ui.toggle(R.string.verification_clipboard_title, R.string.verification_clipboard_note, Sources.clipboard(this)) {
                    prefs.edit().putBoolean(Sources.CLIPBOARD, it).apply()
                    if (!it) Inbox.clear("clipboard")
                },
                ui.toggle(R.string.verification_sms_title, R.string.verification_sms_note, Sources.sms(this), ::setSms),
                ui.toggle(R.string.verification_notifications_title, R.string.verification_notifications_note,
                    Sources.notifications(this), ::setNotifications),
            ),
            ui.group(
                null,
                ui.link(R.string.page_clipboard_sync, null) { startActivity(Intent(this, ClipboardSyncActivity::class.java)) },
            ),
        )
    }

    private fun setSms(enabled: Boolean) {
        if (!enabled) {
            awaitingSms = false
            prefs.edit().putBoolean(Sources.SMS, false).apply()
            Inbox.clear("sms")
            return
        }
        if (!packageManager.hasSystemFeature(PackageManager.FEATURE_TELEPHONY)) {
            Toast.makeText(this, R.string.verification_sms_unavailable, Toast.LENGTH_LONG).show()
            rebuild()
            return
        }
        if (Sources.smsPermission(this)) {
            prefs.edit().putBoolean(Sources.SMS, true).apply()
        } else {
            awaitingSms = true
            requestPermissions(arrayOf(Manifest.permission.RECEIVE_SMS), REQUEST_SMS)
        }
        rebuild()
    }

    override fun onRequestPermissionsResult(requestCode: Int, permissions: Array<out String>, grantResults: IntArray) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (requestCode != REQUEST_SMS) return
        val enabled = awaitingSms && Sources.smsPermission(this)
        awaitingSms = false
        prefs.edit().putBoolean(Sources.SMS, enabled).apply()
        if (!enabled) Toast.makeText(this, R.string.verification_sms_permission_denied, Toast.LENGTH_LONG).show()
        rebuild()
    }

    private fun setNotifications(enabled: Boolean) {
        if (!enabled) {
            awaitingNotifications = false
            prefs.edit().putBoolean(Sources.NOTIFICATIONS, false).apply()
            Inbox.clear("notification")
            return
        }
        if (Sources.notificationAccess(this)) {
            prefs.edit().putBoolean(Sources.NOTIFICATIONS, true).apply()
            return
        }
        awaitingNotifications = true
        val opened = runCatching { startActivity(Intent(android.provider.Settings.ACTION_NOTIFICATION_LISTENER_SETTINGS)) }.isSuccess
        if (!opened) {
            awaitingNotifications = false
            Toast.makeText(this, R.string.verification_notification_settings_unavailable, Toast.LENGTH_LONG).show()
        }
        rebuild()
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

    private companion object {
        const val REQUEST_SMS = 43
    }
}
