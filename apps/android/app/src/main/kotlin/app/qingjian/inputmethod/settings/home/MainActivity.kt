package app.qingjian.inputmethod.settings.home

import android.app.Activity
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
import app.qingjian.inputmethod.settings.widget.SettingsWidgets
import app.qingjian.inputmethod.settings.widget.TabBar
import app.qingjian.inputmethod.settings.widget.Wash
import app.qingjian.inputmethod.theme.AppColors

/** App 的启动页：上面是当前 tab 的标题栏（固定）与内容（渐变底、只有内容滚），底部 tab 栏切 首页 / 生词本 / 我的。 */
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
        ui = SettingsWidgets(this, AppColors.of(this))
        pages = listOf(HomePage(this, ui, ::select), VocabularyPage(this, ui), MePage(this, ui))
        tab = savedInstanceState?.getInt(KEY_TAB) ?: TAB_HOME
        host = FrameLayout(this)
        tabBar = TabBar(
            this, ui,
            listOf(Icon.HOME to R.string.tab_home, Icon.BOOK to R.string.tab_vocabulary, Icon.USER to R.string.tab_me),
            ::select,
        )
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            addView(host, LinearLayout.LayoutParams(MATCH_PARENT, 0, 1f))
            addView(tabBar, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))
        }
        // 内容画到系统栏下面（targetSdk 35 起强制）：渐变铺进状态栏，tab 栏底下让出导航栏
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            root.setOnApplyWindowInsetsListener { _, insets ->
                val bars = insets.getInsets(WindowInsets.Type.systemBars())
                val ime = insets.getInsets(WindowInsets.Type.ime())
                topInset = bars.top
                tabBar.setPadding(0, 0, 0, bars.bottom)
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
            ui.liftOnScroll(header, scroll)
        }))
    }

    /** 标题栏（外面套一层，不冲掉页面自己给标题的留白）让出状态栏；内容区只管左右与底部留白。 */
    private fun pad() {
        header?.setPadding(ui.dp(16), topInset + ui.dp(4), ui.dp(16), 0)
        column?.setPadding(ui.dp(16), ui.dp(4), ui.dp(16), ui.dp(28))
    }

    companion object {
        const val TAB_HOME = 0
        const val TAB_VOCABULARY = 1
        private const val KEY_TAB = "tab"
    }
}
