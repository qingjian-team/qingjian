//! 从新通知的可读内容提取验证码；系统脱敏内容不尝试恢复。

package app.qingjian.inputmethod.verification

import android.app.Notification
import android.service.notification.NotificationListenerService
import android.service.notification.StatusBarNotification
import app.qingjian.inputmethod.clipboard.OtpDetector

class NotificationListener : NotificationListenerService() {
    private val offered = mutableMapOf<String, String>()

    override fun onNotificationPosted(sbn: StatusBarNotification) {
        if (!Sources.notifications(this)) {
            offered.clear()
            Inbox.clear("notification")
            return
        }
        if (sbn.packageName == packageName) return
        offered.remove(sbn.key)?.let { Inbox.remove("notification", it) }
        val notification = sbn.notification
        if (notification.flags and Notification.FLAG_GROUP_SUMMARY != 0) return
        val now = System.currentTimeMillis()
        if (sbn.postTime <= 0 || now - sbn.postTime !in 0..120_000L) return
        val extras = notification.extras
        val parts = buildList {
            listOf(Notification.EXTRA_TITLE, Notification.EXTRA_TEXT, Notification.EXTRA_BIG_TEXT).forEach { key ->
                extras.getCharSequence(key)?.toString()?.takeIf(String::isNotBlank)?.let(::add)
            }
            extras.getCharSequenceArray(Notification.EXTRA_TEXT_LINES)?.forEach {
                it?.toString()?.takeIf(String::isNotBlank)?.let(::add)
            }
        }
        val text = parts.distinct().joinToString(" ").take(4096)
        val code = OtpDetector.detect(text, contextualOnly = true) ?: return
        val id = sbn.key + ":" + sbn.postTime
        offered[sbn.key] = id
        if (offered.size > 32) offered.keys.firstOrNull()?.let(offered::remove)
        Inbox.offer("notification", id, code, masked = true, expiresAt = sbn.postTime + 120_000L)
    }

    override fun onNotificationRemoved(sbn: StatusBarNotification) {
        val id = sbn.key + ":" + sbn.postTime
        if (offered[sbn.key] == id) {
            offered.remove(sbn.key)
            Inbox.remove("notification", id)
        }
    }

    override fun onListenerDisconnected() {
        offered.clear()
        Inbox.clear("notification")
        super.onListenerDisconnected()
    }
}
