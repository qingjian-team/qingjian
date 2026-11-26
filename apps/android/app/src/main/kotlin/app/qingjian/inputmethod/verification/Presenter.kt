//! 将短期验证码接到当前输入框，集中管理刷新、过期和生命周期。

package app.qingjian.inputmethod.verification

import android.inputmethodservice.InputMethodService
import android.os.Handler
import android.os.Looper
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.service.InputPanel

class Presenter(
    private val service: InputMethodService,
    private val panel: () -> InputPanel?,
    private val editingInternally: () -> Boolean,
) {
    private val handler = Handler(Looper.getMainLooper())
    private val refreshLater = Runnable { refresh() }

    fun start() {
        Inbox.onChanged = ::refresh
    }

    fun stop() {
        Inbox.onChanged = null
        hide()
    }

    fun hide() {
        handler.removeCallbacks(refreshLater)
        panel()?.inlineSuggestions?.clearOtp()
    }

    private fun canOffer(): Boolean {
        val info = service.currentInputEditorInfo ?: return false
        return panel() != null && service.isInputViewShown && !editingInternally() &&
            acceptsVerificationCode(info.inputType, info.imeOptions)
    }

    fun refresh() {
        handler.removeCallbacks(refreshLater)
        if (!Sources.clipboard(service)) Inbox.clear("clipboard")
        if (!Sources.sms(service)) Inbox.clear("sms")
        if (!Sources.notifications(service)) Inbox.clear("notification")
        val item = Inbox.current()
        if (!canOffer() || item == null) {
            panel()?.inlineSuggestions?.clearOtp()
            return
        }
        val label = if (item.masked) service.getString(R.string.verification_code_masked)
            else service.getString(R.string.otp_suggestion, item.code)
        panel()?.inlineSuggestions?.showOtp(label) {
            if (canOffer() && Inbox.current() === item &&
                service.currentInputConnection?.commitText(item.code, 1) == true) Inbox.consume(item)
        }
        handler.postDelayed(refreshLater, (item.expiresAt - System.currentTimeMillis()).coerceAtLeast(1))
    }
}
