package app.qingjian.inputmethod.service

import android.content.Context
import android.os.Build
import android.view.View
import android.view.ViewGroup
import android.view.WindowInsets
import android.view.WindowManager
import android.widget.FrameLayout
import android.widget.LinearLayout
import app.qingjian.inputmethod.candidates.CandidateBarView
import app.qingjian.inputmethod.candidates.CandidateGrid
import app.qingjian.inputmethod.candidates.StripSource
import app.qingjian.inputmethod.core.frame.Candidate
import app.qingjian.inputmethod.keyboard.BottomActionsView
import app.qingjian.inputmethod.keyboard.KeyboardView
import app.qingjian.inputmethod.panel.CatalogPanel
import app.qingjian.inputmethod.panel.FunctionPanel
import app.qingjian.inputmethod.panel.KeyboardPickerPanel
import app.qingjian.inputmethod.panel.PanelActions
import app.qingjian.inputmethod.panel.PanelKind
import app.qingjian.inputmethod.panel.VoicePanel
import app.qingjian.inputmethod.panel.clipboard.ClipboardPanel
import app.qingjian.inputmethod.panel.clipboard.PiecesPanel
import app.qingjian.inputmethod.panel.editor.FloatingEditor
import app.qingjian.inputmethod.settings.BottomAction
import app.qingjian.inputmethod.settings.Settings
import app.qingjian.inputmethod.theme.Palette

/**
 * 输入法的整块视图：候选条 + 键盘，全面屏适配开着时底下再接一条操作条，关着只让出导航栏的高度。
 * 面板（[PanelKind]）盖在候选条与键盘那一整块上，高度跟键盘走；底下的键盘留着占位，关面板就露出来。
 */
class InputPanel(
    private val context: Context,
    private val settings: Settings,
    barListener: CandidateBarView.Listener,
    stripSource: StripSource,
    keyboardListener: KeyboardView.Listener,
    private val actions: PanelActions,
    onBottomAction: (BottomAction) -> Unit,
) {
    private val palette = Palette.of(settings.theme, settings.appearance.dark(context))

    val bar = CandidateBarView(context, palette, settings.textSize.scale, stripSource, barListener)
    val keyboard = KeyboardView(context, palette, settings, keyboardListener)

    /** 开着的面板；null 是键盘。 */
    var openPanel: PanelKind? = null
        private set

    private var panelView: View? = null

    // 顶排按键的气泡要画到候选条上，一路都不裁子视图
    private val main = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        clipChildren = false
        clipToPadding = false
        addView(bar)
        addView(keyboard)
    }

    private val stack = Stack(context, main).apply {
        clipChildren = false
        clipToPadding = false
        addView(main)
    }

    private val root = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        setBackgroundColor(palette.board)
        clipChildren = false
        clipToPadding = false
        addView(stack)
        val inset = navigationBarHeight(context)
        if (settings.fullscreenAdapt) {
            addView(BottomActionsView(context, palette, inset, settings.bottomLeft, settings.bottomRight, onBottomAction))
        } else {
            setPadding(0, 0, 0, inset)
        }
    }

    /** 打开 [kind]；拆词要带上那段文字 [text]。 */
    fun open(kind: PanelKind, text: String = "") {
        close()
        val header = bar.barHeight
        val panel = when (kind) {
            PanelKind.FUNCTIONS -> FunctionPanel(context, palette, actions, header, settings)
            PanelKind.KEYBOARDS -> KeyboardPickerPanel(context, palette, actions, header, settings.layout)
            PanelKind.SYMBOLS, PanelKind.EMOJI -> CatalogPanel(context, palette, actions, header, kind)
            PanelKind.CANDIDATES -> CandidateGrid(context, palette, actions, header, settings.textSize.scale)
            PanelKind.CLIPBOARD, PanelKind.PHRASES -> ClipboardPanel(context, palette, actions, header, phrasesFirst = kind == PanelKind.PHRASES)
            PanelKind.PIECES -> PiecesPanel(context, palette, actions, header, text)
            // 语音面板带着回调，由 [openVoice] 盖上
            PanelKind.VOICE -> return
        }
        cover(kind, panel)
    }

    /** 盖上语音面板：右边那块写 [actionLabel]，点各块接回 [actions]。 */
    fun openVoice(actionLabel: String, actions: VoicePanel.Actions): VoicePanel {
        close()
        return VoicePanel(context, palette, actionLabel, actions).also { cover(PanelKind.VOICE, it) }
    }

    private fun cover(kind: PanelKind, panel: View) {
        stack.addView(panel, FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT))
        main.visibility = View.INVISIBLE
        panelView = panel
        openPanel = kind
    }

    val view: View get() = root

    /** 编辑条（常用语编辑框、快捷翻译）摆在最上面（键盘照常可打）；[editor] 为 null 收起。 */
    fun showEditor(editor: View?) {
        root.getChildAt(0)?.takeIf { it is FloatingEditor }?.let(root::removeView)
        editor?.let { root.addView(it, 0) }
    }

    /** 剪贴板面板开着时重读（系统剪贴板刚记进一条）。 */
    fun refreshClipboard() {
        if (openPanel == PanelKind.CLIPBOARD) open(PanelKind.CLIPBOARD)
    }

    /** 候选展开开着时换上这一帧的拼音与候选。 */
    fun showCandidates(preedit: String, candidates: List<Candidate>) {
        (panelView as? CandidateGrid)?.show(preedit, candidates)
    }

    /** 回到键盘；没开面板时什么都不做。 */
    fun close() {
        panelView?.let(stack::removeView)
        panelView = null
        openPanel = null
        main.visibility = View.VISIBLE
    }

    /** 先量键盘那一块，面板一律量成同样高；MATCH_PARENT 在输入法窗口里会撑满整屏。 */
    private class Stack(context: Context, private val main: View) : FrameLayout(context) {
        override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
            measureChild(main, widthMeasureSpec, heightMeasureSpec)
            val height = MeasureSpec.makeMeasureSpec(main.measuredHeight, MeasureSpec.EXACTLY)
            for (i in 0 until childCount) {
                getChildAt(i).takeIf { it !== main }?.measure(widthMeasureSpec, height)
            }
            setMeasuredDimension(MeasureSpec.getSize(widthMeasureSpec), main.measuredHeight)
        }
    }

    /** 系统导航栏 / 手势条的高度。输入法窗口自己收不到这份 inset，从窗口度量里取。 */
    private fun navigationBarHeight(context: Context): Int {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            return context.getSystemService(WindowManager::class.java).currentWindowMetrics.windowInsets
                .getInsetsIgnoringVisibility(WindowInsets.Type.navigationBars()).bottom
        }
        val id = context.resources.getIdentifier("navigation_bar_height", "dimen", "android")
        return if (id > 0) context.resources.getDimensionPixelSize(id) else 0
    }
}
