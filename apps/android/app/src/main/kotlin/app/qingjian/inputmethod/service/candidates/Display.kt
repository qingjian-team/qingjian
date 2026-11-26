//! 电脑候选位图的透明 Android 宿主与应用光标位置订阅。

package app.qingjian.inputmethod.service.candidates

import android.content.Context
import android.graphics.Rect
import android.graphics.RectF
import android.inputmethodservice.InputMethodService
import android.os.Handler
import android.os.Looper
import android.view.View
import android.view.inputmethod.CursorAnchorInfo
import android.view.inputmethod.InputConnection
import app.qingjian.inputmethod.candidates.physical.PhysicalCandidatesView
import app.qingjian.inputmethod.core.Engine
import app.qingjian.inputmethod.core.frame.Candidate
import app.qingjian.inputmethod.settings.Settings

class Display(private val context: Context) {
    private var view: PhysicalCandidatesView? = null

    private var frame: AnchoredFrame? = null

    private var caret: RectF? = null

    private var caretVisible = true

    private var shown = false

    private val handler = Handler(Looper.getMainLooper())

    private var modeLabel: String? = null

    private var modeEnglish = false

    private var onModeExpired: (() -> Unit)? = null

    private val expireMode = Runnable { modeLabel = null; onModeExpired?.invoke() }

    val modeVisible: Boolean get() = modeLabel != null

    fun showMode(label: String, english: Boolean, expired: () -> Unit) {
        handler.removeCallbacks(expireMode)
        modeLabel = label
        modeEnglish = english
        onModeExpired = expired
        handler.postDelayed(expireMode, 900)
    }

    fun cancelMode() {
        handler.removeCallbacks(expireMode)
        modeLabel = null
        onModeExpired = null
    }

    private val origin = IntArray(2)

    private val touch = Rect()

    fun startInput(connection: InputConnection?) {
        caret = null
        caretVisible = true
        frame?.caret = null
        // 不带过滤标志兼容旧应用和 WebView；只读取位置，不记录输入内容。
        connection?.requestCursorUpdates(InputConnection.CURSOR_UPDATE_IMMEDIATE or InputConnection.CURSOR_UPDATE_MONITOR)
    }

    fun updateAnchor(info: CursorAnchorInfo) {
        val x = info.insertionMarkerHorizontal
        val top = info.insertionMarkerTop
        val bottom = info.insertionMarkerBottom
        val points = floatArrayOf(x, top, x, bottom)
        if (points.all { it.isFinite() }) info.matrix.mapPoints(points)
        val flags = info.insertionMarkerFlags
        caretVisible = flags and CursorAnchorInfo.FLAG_HAS_INVISIBLE_REGION == 0 ||
            flags and CursorAnchorInfo.FLAG_HAS_VISIBLE_REGION != 0
        caret = if (points.all { it.isFinite() }) {
            RectF(minOf(points[0], points[2]), minOf(points[1], points[3]),
                maxOf(points[0], points[2]), maxOf(points[1], points[3]))
        } else null
        frame?.caret = caret
        view?.visibility = if (shown && caretVisible) View.VISIBLE else View.GONE
    }

    fun create(engine: Engine, candidates: List<Candidate>, select: (Int) -> Unit, page: (Int) -> Unit): View {
        val candidate = PhysicalCandidatesView(
            context, {
                val dark = Settings.load(context).appearance.dark(context)
                modeLabel?.let { engine.renderPhysicalMode(it, modeEnglish, dark) } ?: engine.renderPhysical(dark)
            }, select, page,
        ).also { it.refresh(if (modeVisible) emptyList() else candidates, modeLabel) }
        view = candidate
        return AnchoredFrame(context, candidate).also { it.caret = caret; frame = it }
    }

    fun refresh(shown: Boolean, candidates: List<Candidate>) {
        this.shown = shown
        view?.let {
            it.visibility = if (shown && caretVisible) View.VISIBLE else View.GONE
            if (shown) it.refresh(if (modeVisible) emptyList() else candidates, modeLabel)
        }
    }

    fun computeInsets(outInsets: InputMethodService.Insets, windowHeight: Int) {
        outInsets.contentTopInsets = windowHeight
        outInsets.visibleTopInsets = windowHeight
        outInsets.touchableInsets = InputMethodService.Insets.TOUCHABLE_INSETS_REGION
        outInsets.touchableRegion.setEmpty()
        if (modeVisible) return
        val candidate = view?.takeIf { it.isShown && it.width > 0 && it.height > 0 } ?: return
        candidate.getLocationInWindow(origin)
        candidate.contentBounds(touch)
        touch.offset(origin[0], origin[1])
        outInsets.touchableRegion.set(touch)
    }
}
