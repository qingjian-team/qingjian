package app.qingjian.inputmethod.settings.home

import android.content.Context
import android.content.Intent
import android.provider.Settings as SystemSettings
import android.view.inputmethod.InputMethodManager

/** 青简在系统里启用了没有、是不是当前输入法；以及去启用、弹选择框这两个动作。 */
data class ImeState(val enabled: Boolean, val current: Boolean) {
    val ready get() = enabled && current

    companion object {
        fun of(context: Context): ImeState {
            val manager = context.getSystemService(InputMethodManager::class.java)
            val enabled = manager.enabledInputMethodList.any { it.packageName == context.packageName }
            val current = SystemSettings.Secure.getString(
                context.contentResolver, SystemSettings.Secure.DEFAULT_INPUT_METHOD,
            ).orEmpty().startsWith("${context.packageName}/")
            return ImeState(enabled, current)
        }

        fun openSystemSettings(context: Context) =
            context.startActivity(Intent(SystemSettings.ACTION_INPUT_METHOD_SETTINGS))

        fun showPicker(context: Context) =
            context.getSystemService(InputMethodManager::class.java).showInputMethodPicker()
    }
}
