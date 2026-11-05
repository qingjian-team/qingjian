package app.qingjian.inputmethod.settings.widget

import android.content.Context
import android.view.Gravity
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.LinearLayout
import app.qingjian.inputmethod.panel.base.PanelIcons.Icon

/** 底部 tab 栏：栏底色、顶上一道细线，图标 + 小字，选中的用品牌色。 */
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
        orientation = HORIZONTAL
        // 底下让出的导航栏那一截也铺同一个底色
        background = ui.barBackground(bottomLine = false)
        setPadding(0, ui.dp(6), 0, ui.dp(4))
    }

    private fun render() {
        removeAllViews()
        tabs.forEachIndexed { index, (icon, label) ->
            val on = index == selected
            val color = if (on) ui.colors.brand else ui.colors.secondary
            addView(LinearLayout(context).apply {
                orientation = VERTICAL
                gravity = Gravity.CENTER_HORIZONTAL
                setPadding(0, ui.dp(2), 0, ui.dp(4))
                isSelected = on
                contentDescription = context.getString(label)
                setOnClickListener { onSelect(index) }
                addView(ui.icon(icon, color, 25f))
                addView(ui.text(context.getString(label), 10.5f, color, bold = on).apply {
                    gravity = Gravity.CENTER
                    setPadding(0, ui.dp(3), 0, 0)
                })
            }, LayoutParams(0, WRAP_CONTENT, 1f))
        }
    }
}
