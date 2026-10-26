package app.qingjian.inputmethod.settings

import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.panel.base.PanelIcons.Icon
import app.qingjian.inputmethod.settings.page.KeyboardSettingsActivity

/**
 * 全面屏底栏左右两个角各放什么（键盘设置里选，两边不能一样，见 [KeyboardSettingsActivity]）。
 * [NONE] 那一角空着。
 */
enum class BottomAction(val label: Int, val icon: Icon?) {
    SWITCH_INPUT_METHOD(R.string.bottom_switch, Icon.GLOBE),
    VOICE(R.string.bottom_voice, Icon.MIC),
    CLIPBOARD(R.string.bottom_clipboard, Icon.CLIPBOARD),
    HIDE_KEYBOARD(R.string.bottom_hide, Icon.HIDE),
    NONE(R.string.bottom_none, null),
    ;

    companion object {
        fun of(name: String?, fallback: BottomAction): BottomAction = entries.firstOrNull { it.name == name } ?: fallback
    }
}
