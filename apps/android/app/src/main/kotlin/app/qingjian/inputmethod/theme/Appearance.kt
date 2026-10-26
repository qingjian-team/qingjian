package app.qingjian.inputmethod.theme

import android.content.Context
import android.content.res.Configuration
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

    companion object {
        fun of(key: String?) = entries.firstOrNull { it.key == key } ?: SYSTEM
    }
}
