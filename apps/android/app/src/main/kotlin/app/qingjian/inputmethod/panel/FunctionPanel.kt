package app.qingjian.inputmethod.panel

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.RectF
import app.qingjian.inputmethod.panel.base.PanelIcons.Icon
import app.qingjian.inputmethod.panel.base.PanelView
import app.qingjian.inputmethod.panel.base.Tabs
import app.qingjian.inputmethod.settings.Settings
import app.qingjian.inputmethod.theme.Palette

/**
 * 点工具栏 logo 进的功能面板：「常用功能」是图标宫格，「键盘设置」是就地改的开关；
 * 右上角进完整设置页。整块圆角卡片垫在键盘底色上。
 */
@SuppressLint("ViewConstructor")
class FunctionPanel(
    context: Context,
    palette: Palette,
    actions: PanelActions,
    headerHeight: Float,
    private val settings: Settings,
) : PanelView(context, palette, actions, headerHeight) {
    /** 一格功能；[glyph] 不为 null 时画字不画图标，[on] 是开关类功能开着（品牌色加对勾，同键盘选择）。 */
    private class Feature(
        val icon: Icon?,
        val label: String,
        val glyph: String? = null,
        val on: Boolean = false,
        val run: PanelActions.() -> Unit,
    )

    private class Toggle(val key: String, val label: String, val note: String, val on: Boolean)

    private val features = listOf(
        Feature(Icon.EMOJI, "表情") { openPanel(PanelKind.EMOJI) },
        Feature(Icon.SYMBOLS, "符号") { openPanel(PanelKind.SYMBOLS) },
        Feature(Icon.KEYBOARD, "切换键盘") { openPanel(PanelKind.KEYBOARDS) },
        Feature(null, "繁体", glyph = "繁", on = settings.traditional) {
            setToggle(Settings.KEY_TRADITIONAL, !settings.traditional)
        },
        Feature(Icon.RESIZE, "键盘高度") { startResize() },
        Feature(Icon.GLOBE, "切换输入法") { switchInputMethod() },
        Feature(Icon.SETTINGS, "设置") { openSettings() },
    )

    private val toggles
        get() = listOf(
            Toggle(Settings.KEY_HAPTIC, "按键震动", "", settings.haptic),
            Toggle(Settings.KEY_POPUP, "按键气泡", "按下字母时在上面放大显示", settings.keyPopup),
            Toggle(Settings.KEY_FULLSCREEN, "全面屏适配", "手势条那一截不留白，两个角各放一个键", settings.fullscreenAdapt),
            Toggle(Settings.KEY_KEEP_CHINESE, "换语言时保留中文", "开着时总在中文和选的语言之间切", settings.keepChinese),
        )

    private val tabs = Tabs(ink, scrollable(true) { x, y -> y < headerHeight && x > ink.dp(BACK_WIDTH_DP) }, Tabs.Style.UNDERLINE)
        .apply {
            labels = listOf("常用功能", "键盘设置")
            selected = lastTab
        }

    private val card = RectF()
    private val knob = RectF()

    private val columns = 5

    override fun onDraw(canvas: Canvas) {
        drawHeader(canvas, tabs, Icon.SETTINGS)
        card.set(ink.dp(10f), headerHeight, width - ink.dp(10f), height - ink.dp(8f))
        ink.round(canvas, card, 16f, palette.surface)
        if (tabs.selected == 0) drawFeatures(canvas) else drawToggles(canvas)
    }

    private fun cellWidth() = card.width() / columns

    private fun cellHeight() = minOf(ink.dp(86f), card.height() / 2)

    private fun drawFeatures(canvas: Canvas) {
        val top = card.top + ink.dp(8f)
        features.forEachIndexed { i, feature ->
            val cx = card.left + cellWidth() * (i % columns + 0.5f)
            val cy = top + cellHeight() * (i / columns + 0.5f)
            val color = if (feature.on) palette.brand else palette.text
            feature.icon?.let { ink.icon(canvas, it, cx, cy - ink.dp(12f), 30f, color) }
            feature.glyph?.let { ink.text(canvas, it, 24f, color, cx, cy - ink.dp(12f), bold = true) }
            if (feature.on) drawBadge(canvas, cx + ink.dp(18f), cy - ink.dp(26f))
            val label = if (feature.on) palette.brand else palette.gloss
            ink.text(canvas, feature.label, 13f, label, cx, cy + ink.dp(20f), maxWidth = cellWidth() - ink.dp(4f))
        }
    }

    private fun drawBadge(canvas: Canvas, cx: Float, cy: Float) {
        ink.fill.color = palette.brand
        canvas.drawCircle(cx, cy, ink.dp(7.5f), ink.fill)
        ink.icon(canvas, Icon.CHECK, cx, cy, 13f, palette.onBrand)
    }

    private fun rowHeight() = card.height() / toggles.size

    private fun drawToggles(canvas: Canvas) {
        toggles.forEachIndexed { i, toggle ->
            val top = card.top + rowHeight() * i
            val cy = top + rowHeight() / 2
            val left = card.left + ink.dp(20f)
            if (toggle.note.isEmpty()) {
                drawLeft(canvas, toggle.label, 16f, palette.text, left, cy)
            } else {
                drawLeft(canvas, toggle.label, 16f, palette.text, left, cy - ink.dp(10f))
                drawLeft(canvas, toggle.note, 12f, palette.gloss, left, cy + ink.dp(12f))
            }
            drawSwitch(canvas, card.right - ink.dp(44f), cy, toggle.on)
            if (i > 0) ink.line(canvas, left, top, card.right - ink.dp(20f), top, palette.faint)
        }
    }

    private fun drawLeft(canvas: Canvas, value: String, sp: Float, color: Int, left: Float, cy: Float) {
        val width = ink.measure(value, sp)
        ink.text(canvas, value, sp, color, left + width / 2, cy)
    }

    private fun drawSwitch(canvas: Canvas, cx: Float, cy: Float, on: Boolean) {
        knob.set(cx - ink.dp(22f), cy - ink.dp(13f), cx + ink.dp(22f), cy + ink.dp(13f))
        ink.round(canvas, knob, 13f, if (on) palette.brand else palette.functionalKey)
        val kx = if (on) cx + ink.dp(9f) else cx - ink.dp(9f)
        ink.fill.color = palette.surface
        canvas.drawCircle(kx, cy, ink.dp(10f), ink.fill)
    }

    override fun onTap(x: Float, y: Float) {
        when {
            isRightIcon(x, y) -> actions.openSettings()
            y < headerHeight -> tabs.indexAt(x)?.let {
                tabs.selected = it
                lastTab = it
            }

            !card.contains(x, y) -> Unit
            tabs.selected == 0 -> {
                val row = ((y - card.top - ink.dp(8f)) / cellHeight()).toInt()
                val column = ((x - card.left) / cellWidth()).toInt()
                features.getOrNull(row * columns + column)?.run?.invoke(actions)
            }

            else -> toggles.getOrNull(((y - card.top) / rowHeight()).toInt())?.let {
                actions.setToggle(it.key, !it.on)
            }
        }
    }

    private companion object {
        /** 上次停在哪一页；改开关会重建键盘，回来时还在这页。 */
        var lastTab = 0
    }
}
