package app.qingjian.inputmethod.settings

import app.qingjian.inputmethod.panel.base.PanelIcons.Icon

/**
 * 工具栏中间那排能放的工具（#777）。放哪些、什么顺序由用户在功能面板的「定制工具栏」里勾，按勾的先后排；
 * 没定制过是 [DEFAULT]。两头的 logo 与收起键盘固定，不在这里。
 */
enum class ToolbarTool(val label: String, val icon: Icon) {
    EMOJI("表情", Icon.EMOJI),
    KEYBOARDS("切换键盘", Icon.KEYBOARD),
    VOICE("语音", Icon.MIC),
    SYMBOLS("符号", Icon.SYMBOLS),
    CLIPBOARD("剪贴板", Icon.CLIPBOARD),
    TRANSLATE("翻译", Icon.TRANSLATE),
    RESIZE("键盘高度", Icon.RESIZE),
    PHRASES("常用语", Icon.PHRASES),
    SWITCH_INPUT_METHOD("切换输入法", Icon.GLOBE),
    SETTINGS("设置", Icon.SETTINGS),
    ;

    companion object {
        /** 没定制过的工具栏，也是「恢复默认」回到的样子。 */
        val DEFAULT = listOf(EMOJI, KEYBOARDS, VOICE, SYMBOLS, CLIPBOARD, TRANSLATE, RESIZE)

        /** 存的是逗号分隔的名字；没存过为 null，用 [DEFAULT]；空串是全部移除了。认不出的名字（降级装回旧版）跳过。 */
        fun parse(value: String?): List<ToolbarTool> {
            if (value == null) return DEFAULT
            return value.split(',').mapNotNull { name -> entries.firstOrNull { it.name == name } }.distinct()
        }

        fun format(tools: List<ToolbarTool>): String = tools.joinToString(",") { it.name }
    }
}
