package app.qingjian.inputmethod.theme

import android.app.UiModeManager
import android.content.Context
import android.content.res.Configuration
import android.os.Build
import app.qingjian.inputmethod.R

/** 深浅色：跟随系统或固定一种，同 PC 的 `[general] appearance`。主题锁定了外观时以主题为准（Rust 侧处理）。 */
enum class Appearance(val key: String, val label: Int) {
    SYSTEM("system", R.string.appearance_system),
    LIGHT("light", R.string.appearance_light),
    DARK("dark", R.string.appearance_dark);

    /** 这时该用深色吗。 */
    fun dark(context: Context): Boolean = when (this) {
        SYSTEM -> context.resources.configuration.uiMode and
            Configuration.UI_MODE_NIGHT_MASK == Configuration.UI_MODE_NIGHT_YES
        LIGHT -> false
        DARK -> true
    }

    /**
     * 让 App 自己的界面也按这个深浅色（Android 12 起有按应用的夜间模式，系统设置不动；更早的版本 App 界面仍跟随系统）。
     * 值没变时系统什么也不做，变了会重建 App 的页面。
     */
    fun applyToApp(context: Context) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.S) return
        val manager = context.getSystemService(UiModeManager::class.java) ?: return
        val mode = when (this) {
            SYSTEM -> UiModeManager.MODE_NIGHT_AUTO
            LIGHT -> UiModeManager.MODE_NIGHT_NO
            DARK -> UiModeManager.MODE_NIGHT_YES
        }
        manager.setApplicationNightMode(mode)
    }

    companion object {
        fun of(key: String?) = entries.firstOrNull { it.key == key } ?: SYSTEM
    }
}
