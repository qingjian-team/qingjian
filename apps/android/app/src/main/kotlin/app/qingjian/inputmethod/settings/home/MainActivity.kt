package app.qingjian.inputmethod.settings.home

import android.app.Activity
import android.content.Intent
import android.os.Build
import android.os.Bundle
import android.view.View
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.view.WindowInsets
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.ScrollView
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.panel.base.PanelIcons.Icon
import app.qingjian.inputmethod.settings.Settings
import app.qingjian.inputmethod.settings.widget.SettingsWidgets
import app.qingjian.inputmethod.settings.widget.TabBar
import app.qingjian.inputmethod.settings.widget.Wash
import app.qingjian.inputmethod.theme.AppColors

/** App 的启动页：上面是当前 tab 的标题栏（固定）与内容（只有内容滚），底部 tab 栏切 首页 / 生词本 / 设置。 */
class MainActivity : Activity() {
    private lateinit var ui: SettingsWidgets
    private lateinit var pages: List<AppPage>
    private lateinit var tabBar: TabBar
    private lateinit var host: FrameLayout
    private var tab = TAB_HOME
    private var topInset = 0
    private var header: View? = null
    private var column: View? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // 进 App 时按「外观」对一次 App 自己的深浅色：设置可能是别处改的（值没变时系统不做事）
        Settings.load(this).appearance.applyToApp(this)
        ui = SettingsWidgets(this, AppColors.of(this))
        pages = listOf(HomePage(this, ui, ::select), VocabularyPage(this, ui), SettingsPage(this, ui))
        tab = savedInstanceState?.getInt(KEY_TAB) ?: intent.getIntExtra(EXTRA_TAB, TAB_HOME)
        host = FrameLayout(this)
        tabBar = TabBar(
            this, ui,
            listOf(Icon.HOME to R.string.tab_home, Icon.BOOK to R.string.tab_vocabulary, Icon.SETTINGS to R.string.tab_settings),
            ::select,
        )
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            addView(host, LinearLayout.LayoutParams(MATCH_PARENT, 0, 1f))
            addView(tabBar, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))
        }
        // 内容画到系统栏下面（targetSdk 35 起强制）：顶栏让出状态栏与刘海（高度由系统给），tab 栏底下让出导航栏
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            root.setOnApplyWindowInsetsListener { _, insets ->
                val bars = insets.getInsets(WindowInsets.Type.systemBars())
                val ime = insets.getInsets(WindowInsets.Type.ime())
                topInset = bars.top
                tabBar.setPadding(0, ui.dp(6), 0, bars.bottom + ui.dp(4))
                host.setPadding(0, 0, 0, maxOf(0, ime.bottom - bars.bottom - tabBar.height))
                // 只调边距不重建：insets 变化很频繁（键盘起落），重建会吞掉正在进行的点击、冲掉试打框
                pad()
                insets
            }
        } else {
            root.fitsSystemWindows = true
        }
        setContentView(root)
        show()
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        select(intent.getIntExtra(EXTRA_TAB, tab))
    }

    override fun onSaveInstanceState(outState: Bundle) {
        super.onSaveInstanceState(outState)
        outState.putInt(KEY_TAB, tab)
    }

    /** 从系统设置回来、选输入法的弹窗关掉、打完字回来：数据变了才重建这一页。 */
    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (hasFocus && pages[tab].stale()) show()
    }

    private fun select(index: Int) {
        if (index == tab) return
        tab = index
        show()
    }

    private fun show() {
        tabBar.selected = tab
        val page = pages[tab]
        val header = FrameLayout(this).apply { addView(page.header(), MATCH_PARENT, WRAP_CONTENT) }
        val column = page.build()
        this.header = header
        this.column = column
        pad()
        host.removeAllViews()
        host.addView(Wash.behind(this, ui.colors, LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            addView(header)
            val scroll = ScrollView(this@MainActivity).apply {
                isVerticalScrollBarEnabled = false
                addView(column)
            }
            addView(scroll, LinearLayout.LayoutParams(MATCH_PARENT, 0, 1f))
            header.background = ui.barBackground(bottomLine = true)
        }))
    }

    /** 标题栏（外面套一层，不冲掉页面自己给标题的留白）让出状态栏；内容区只管左右与底部留白。 */
    private fun pad() {
        header?.setPadding(ui.dp(4), topInset, ui.dp(4), 0)
        column?.setPadding(ui.dp(16), 0, ui.dp(16), ui.dp(28))
    }

    companion object {
        const val TAB_HOME = 0
        const val TAB_VOCABULARY = 1
        const val TAB_SETTINGS = 2
        private const val KEY_TAB = "tab"

        /** 从键盘的设置按钮进来时直接落在设置 tab。 */
        const val EXTRA_TAB = "app.qingjian.inputmethod.TAB"
    }
}
