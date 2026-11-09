package app.qingjian.inputmethod.translate

import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.inputmethod.InputConnection
import app.qingjian.inputmethod.core.Engine
/**
 * 快捷翻译的流程：翻译框的原文停顿 [DEBOUNCE_MS] 后交 Core 发云端，轮询取译文，
 * 译文用组合文字（未确认）放进外面的输入框 [output]，原文再变就换掉；[finish] 确认上屏，[discard] 撤掉。
 * 过期的译文 Core 自己丢（只留最新一条翻译请求）。
 */
class QuickTranslate(private val engine: Engine, private val output: () -> InputConnection?) {
    private val handler = Handler(Looper.getMainLooper())

    /** 开着的翻译条；状态字写到它上面。 */
    var bar: TranslateBar? = null

    private var source = ""
    private var sentAt = 0L

    /** 原文变了：作废在飞的，停顿后再发；清空了就撤掉译文。 */
    fun onSource(text: String) {
        stop()
        source = text
        show(null)
        if (text.isEmpty()) {
            output()?.setComposingText("", 1)
        } else {
            handler.postDelayed(send, DEBOUNCE_MS)
        }
    }

    private val send = Runnable {
        if (engine.translate(source)) {
            sentAt = SystemClock.uptimeMillis()
            show("翻译中…")
            handler.postDelayed(poll, POLL_MS)
        } else {
            show("翻译失败", failed = true)
        }
    }

    private val poll: Runnable = Runnable {
        val translated = engine.pollTranslation()
        when {
            translated != null -> {
                // 出错、解析不了回包、模型没给内容，Core 都回空串
                if (translated.isEmpty()) show("翻译失败", failed = true) else show(null)
                if (translated.isNotEmpty()) output()?.setComposingText(translated, 1)
            }
            SystemClock.uptimeMillis() - sentAt > GIVE_UP_MS -> show("没有回应", failed = true)
            else -> handler.postDelayed(poll, POLL_MS)
        }
    }

    /** 翻译条右侧的状态字；[failed] 时画成红字。 */
    private fun show(status: String?, failed: Boolean = false) {
        bar?.status = status
        bar?.failed = failed
    }

    private fun stop() {
        handler.removeCallbacks(send)
        handler.removeCallbacks(poll)
        engine.cancelTranslation()
    }

    /** 确认：输入框里的译文落定。 */
    fun finish() {
        stop()
        output()?.finishComposingText()
        bar = null
    }

    /** 取消：撤掉输入框里的译文。 */
    fun discard() {
        stop()
        output()?.run {
            setComposingText("", 1)
            finishComposingText()
        }
        bar = null
    }

    private companion object {
        const val DEBOUNCE_MS = 450L
        const val POLL_MS = 100L

        /** Rust 侧请求超时是 10 秒，到时会回空结果；这里只是兜底。 */
        const val GIVE_UP_MS = 15_000L
    }
}
