package app.qingjian.inputmethod.settings.widget

import android.content.Context
import android.view.Gravity
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.LinearLayout
import app.qingjian.inputmethod.panel.base.PanelIcons.Icon

/** 底部 tab 栏：白底，顶上一道细线与阴影，图标 + 字，选中的用品牌色。 */
class TabBar(
    context: Context,
    private val ui: SettingsWidgets,
    private val tabs: List<Pair<Icon, Int>>,
    private val onSelect: (Int) -> Unit,
) : LinearLayout(context) {
    var selected = -1
        set(value) {
            field = value
            render()
        }

    init {
        orientation = VERTICAL
        // 底下让出的导航栏那一截也是白的
        setBackgroundColor(ui.colors.surface)
        // 往上投一道阴影，内容滚到它底下时分得出层次
        elevation = ui.dp(8).toFloat()
    }

    private fun render() {
        removeAllViews()
        addView(android.view.View(context).apply { setBackgroundColor(ui.colors.divider) }, LayoutParams(LayoutParams.MATCH_PARENT, maxOf(1, ui.dp(1) / 2)))
        addView(LinearLayout(context).apply {
            orientation = HORIZONTAL
            setBackgroundColor(ui.colors.surface)
            setPadding(0, ui.dp(6), 0, ui.dp(4))
            tabs.forEachIndexed { index, (icon, label) ->
                val color = if (index == selected) ui.colors.brand else ui.colors.secondary
                addView(LinearLayout(context).apply {
                    orientation = VERTICAL
                    gravity = Gravity.CENTER_HORIZONTAL
                    setPadding(0, ui.dp(4), 0, ui.dp(4))
                    setOnClickListener { onSelect(index) }
                    addView(ui.icon(icon, color, 26f))
                    addView(ui.text(context.getString(label), 11f, color, bold = index == selected).apply {
                        gravity = Gravity.CENTER
                        setPadding(0, ui.dp(4), 0, 0)
                    })
                }, LayoutParams(0, WRAP_CONTENT, 1f))
            }
        })
    }
}
