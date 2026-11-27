//! 物理候选的 IME 窗口切换与显示请求，绘制交给 Display。

package app.qingjian.inputmethod.service.candidates

import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.Drawable
import android.inputmethodservice.InputMethodService
import android.view.View
import android.view.ViewGroup
import android.view.Window
import app.qingjian.inputmethod.core.Engine
import app.qingjian.inputmethod.core.frame.Candidate
import app.qingjian.inputmethod.service.input.Composer
import app.qingjian.inputmethod.service.input.physical.Action

class WindowHost(
    private val service: InputMethodService,

    private val engine: Engine,

    private val composer: Composer,

    private val active: () -> Boolean,

    private val touchCandidates: (String, List<Candidate>) -> Unit,
) {
    val display = Display(service)

    var shown = false
        private set

    var installed = false
        private set

    private val softBackground: Drawable? = service.window.window?.decorView?.background

    fun createView(candidates: List<Candidate>): View {
        installed = true
        service.window.window?.setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
        service.window.window?.setLayout(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT)
        return display.create(engine, candidates, { composer.select(it) }, {
            composer.onPhysical(if (it < 0) Action.PREVIOUS_PAGE else Action.NEXT_PAGE, 0, false)
        })
    }

    fun refresh(preedit: String, candidates: List<Candidate>) {
        if (!active()) display.cancelMode()
        val next = active() && !composer.directInput && (preedit.isNotEmpty() || candidates.isNotEmpty() || display.modeVisible)
        val changed = shown != next
        shown = next
        if (shown) {
            if (!installed) service.setInputView(createView(candidates))
            display.refresh(true, candidates)
            if (changed) {
                service.updateFullscreenMode()
                service.updateInputViewShown()
            }
            // Android 16 客户端与 IME 状态可能不同步，两端一起更新。
            if (changed || !service.isInputViewShown) {
                service.requestShowSelf(0)
                if (!service.isInputViewShown) service.showWindow(true)
            }
        } else {
            display.refresh(false, emptyList())
            if (changed && installed) hide()
            if (!active()) touchCandidates(preedit, candidates)
        }
    }

    fun restoreSoftView() {
        installed = false
        service.window.window?.setBackgroundDrawable(softBackground)
        service.window.window?.setLayout(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT)
    }

    fun configure(win: Window) {
        if (installed) win.setLayout(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT)
    }

    /** 排队的旧显示请求到达时，不能在物理组字结束后闪出软键盘。 */
    fun rejectStaleShow(): Boolean {
        if (!installed || !active()) return false
        hide()
        return true
    }

    fun hide() {
        service.requestHideSelf(0)
        if (service.isInputViewShown) service.hideWindow()
    }

    fun finish() {
        display.cancelMode()
        display.startInput(null)
        display.refresh(false, emptyList())
        shown = false
    }
}
