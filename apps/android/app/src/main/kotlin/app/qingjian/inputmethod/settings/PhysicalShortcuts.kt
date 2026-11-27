//! 物理键盘快捷键设置，沿用桌面快捷键的语义。

package app.qingjian.inputmethod.settings

import android.content.Context

/** 实体键盘快捷键：配置词与桌面 shortcut 分节一致，不引入平台 crate 的 TLS 依赖。 */
data class PhysicalShortcuts(
    val shift: Boolean,

    val control: Boolean,

    val ctrlAltSpace: Boolean,

    val enterCandidate: Boolean,
) {
    companion object {
        const val KEY_SHIFT = "shortcut.switch_mode.shift"

        const val KEY_CONTROL = "shortcut.switch_mode.control"

        const val KEY_CTRL_ALT_SPACE = "shortcut.switch_mode.ctrl+alt+space"

        const val KEY_ENTER_CANDIDATE = "shortcut.commit_candidate.enter"

        fun load(context: Context): PhysicalShortcuts = Settings.preferences(context).let {
            PhysicalShortcuts(
                it.getBoolean(KEY_SHIFT, true), it.getBoolean(KEY_CONTROL, false),
                it.getBoolean(KEY_CTRL_ALT_SPACE, false), it.getBoolean(KEY_ENTER_CANDIDATE, false),
            )
        }
    }
}
