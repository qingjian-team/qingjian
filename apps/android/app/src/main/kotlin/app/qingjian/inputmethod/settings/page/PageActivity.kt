package app.qingjian.inputmethod.settings.page

import android.app.Activity
import android.os.Build
import android.os.Bundle
import android.view.View
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.view.WindowInsets
import android.widget.LinearLayout
import android.widget.ScrollView
import app.qingjian.inputmethod.settings.Settings
import app.qingjian.inputmethod.settings.widget.SettingsWidgets
import app.qingjian.inputmethod.settings.widget.Wash
import app.qingjian.inputmethod.theme.AppColors

/**
 * App 二级页的底子：与首页同样的渐变灰底，顶栏固定、下面一列卡片单独滚，自己让出系统栏（targetSdk 35 起内容强制画到系统栏下面）。
 * 子类给出 [title] 与 [content] 里的各块，[rebuild] 重画整页（滚动位置保留）。
 */
abstract class PageActivity : Activity() {
    protected lateinit var ui: SettingsWidgets
    protected val prefs by lazy { Settings.preferences(this) }
    private var scroll: ScrollView? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        ui = SettingsWidgets(this, AppColors.of(this))
        rebuild()
    }

    protected abstract val title: Int

    protected abstract fun content(): List<View>

    protected fun rebuild() {
        val scrolled = scroll?.scrollY ?: 0
        val column = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(ui.dp(16), ui.dp(4), ui.dp(16), ui.dp(32))
            content().forEach(::addView)
        }
        val scroll = ScrollView(this).apply {
            isVerticalScrollBarEnabled = false
            addView(column)
            post { scrollTo(0, scrolled) }
        }
        this.scroll = scroll
        val bar = ui.topBar(getString(title)) { finish() }
        ui.liftOnScroll(bar, scroll)
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            addView(bar)
            addView(scroll, LinearLayout.LayoutParams(MATCH_PARENT, 0, 1f))
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            root.setOnApplyWindowInsetsListener { view, insets ->
                val bars = insets.getInsets(WindowInsets.Type.systemBars() or WindowInsets.Type.ime())
                // 状态栏那一截算顶栏的，浮起来时白底一起铺上去
                view.setPadding(bars.left, 0, bars.right, bars.bottom)
                bar.setPadding(ui.dp(16), bars.top + ui.dp(4), ui.dp(16), ui.dp(8))
                insets
            }
        } else {
            root.fitsSystemWindows = true
        }
        setContentView(Wash.behind(this, ui.colors, root))
    }

    /** 卡片之间的间距。 */
    protected fun spaced(view: View, topDp: Int = 12) = view.apply {
        layoutParams = LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT).apply { topMargin = ui.dp(topDp) }
    }
}
