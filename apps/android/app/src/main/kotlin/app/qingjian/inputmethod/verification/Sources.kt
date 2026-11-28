//! 验证码来源的独立开关；系统权限未授予时不能视为已启用。

package app.qingjian.inputmethod.verification

import android.Manifest
import android.app.NotificationManager
import android.content.ComponentName
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import app.qingjian.inputmethod.settings.Settings

object Sources {
    const val CLIPBOARD = "verification_clipboard"
    const val SMS = "verification_sms"
    const val NOTIFICATIONS = "verification_notifications"

    fun clipboard(context: Context): Boolean = Settings.preferences(context).getBoolean(CLIPBOARD, true)

    fun smsPermission(context: Context): Boolean =
        context.checkSelfPermission(Manifest.permission.RECEIVE_SMS) == PackageManager.PERMISSION_GRANTED

    fun sms(context: Context): Boolean = Settings.preferences(context).getBoolean(SMS, false) && smsPermission(context)

    fun notificationAccess(context: Context): Boolean {
        val component = ComponentName(context, NotificationListener::class.java)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O_MR1) {
            return context.getSystemService(NotificationManager::class.java)
                ?.isNotificationListenerAccessGranted(component) == true
        }
        // API 26 尚无 NotificationManager 的查询方法，读取系统公开的授权组件列表。
        val enabled = android.provider.Settings.Secure.getString(context.contentResolver, "enabled_notification_listeners")
        return enabled?.split(':')?.any { ComponentName.unflattenFromString(it) == component } == true
    }

    fun notifications(context: Context): Boolean =
        Settings.preferences(context).getBoolean(NOTIFICATIONS, false) && notificationAccess(context)
}
