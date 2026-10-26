package app.qingjian.inputmethod.voice

import android.content.Context
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.core.Engine
import app.qingjian.inputmethod.panel.VoicePanel

/**
 * 语音输入的唯一入口：长按空格、工具栏话筒、全面屏底栏的语音键都走 [start]。
 * 挑识别方式、查录音权限、开关语音面板；听出来的字边说边以编辑中的文字放进输入框，说完经 Core 落定。
 * 识别本身在 [VoiceRecognizer] 的实现里。
 */
class VoiceInput(private val context: Context, private val engine: Engine, private val host: Host) :
    VoiceRecognizer.Listener, VoicePanel.Actions {
    interface Host {
        /** 盖上语音面板，点它的各块接回 [actions]；键盘没建起来时为 null。 */
        fun openVoicePanel(actions: VoicePanel.Actions): VoicePanel?

        fun closeVoicePanel()

        /** 开始说话前：组句落下，识别出的字接在后面。 */
        fun prepareVoice()

        /** 说话途中听出来的字：放进输入框、处在编辑中；空串撤掉。 */
        fun composeVoice(text: String)

        /** 最终结果上屏（经 Core），替换掉编辑中的那段。 */
        fun commitVoice(text: String)

        /** 执行输入框的回车动作（发送、搜索）；没有动作时什么都不做。 */
        fun performVoiceAction()

        fun hideKeyboard()

        fun toast(message: String)
    }

    /** 最终结果上屏之后还要做的事。 */
    private enum class After { NOTHING, ACTION, HIDE }

    private var recognizer: VoiceRecognizer? = null
    private var panel: VoicePanel? = null
    private var after = After.NOTHING

    val active: Boolean get() = recognizer != null

    /** 「语音识别」配好了就用 Rust 侧的识别方，否则是 [UnavailableRecognizer]。 */
    private fun pick(): VoiceRecognizer = when {
        engine.voiceAvailable -> EngineRecognizer(engine)
        else -> UnavailableRecognizer
    }

    /** 开始说话；[holding] 是按着空格说。返回开没开起来：不能用、要先申请权限都是 false。 */
    fun start(holding: Boolean): Boolean {
        if (active) return true
        val picked = pick()
        if (!picked.available) {
            host.toast(context.getString(R.string.voice_unavailable))
            return false
        }
        if (picked.needsMicrophone && !MicrophonePermissionActivity.granted(context)) {
            // 弹系统的权限框；给了就直接开始（按着空格的那次手指早抬起了，按点按算）
            MicrophonePermissionActivity.request(context) { granted ->
                if (granted) start(holding = false) else host.toast(context.getString(R.string.voice_no_permission))
            }
            return false
        }
        host.prepareVoice()
        val panel = host.openVoicePanel(this) ?: return false
        panel.holding = holding
        this.panel = panel
        after = After.NOTHING
        recognizer = picked
        picked.start(this)
        return true
    }

    /** 按着空格说时手指挪到了屏幕上的这一点。 */
    fun drag(screenX: Float, screenY: Float) {
        panel?.takeIf { it.holding }?.track(screenX, screenY)
    }

    /** 按着空格的手指抬起：松在「撤回输入」上撤回，松在右边那块上上屏并执行回车动作，别处只是说完。[cancel] 是手指被系统收走。 */
    fun release(cancel: Boolean) {
        val panel = panel?.takeIf { it.holding } ?: return
        panel.holding = false
        when {
            cancel || panel.target == VoicePanel.Target.CANCEL -> onCancel()
            else -> onFinish(act = panel.target == VoicePanel.Target.ACTION, hide = false)
        }
    }

    override fun onFinish(act: Boolean, hide: Boolean) {
        val recognizer = recognizer ?: return
        after = when {
            act -> After.ACTION
            hide -> After.HIDE
            else -> After.NOTHING
        }
        panel?.recognizing = true
        recognizer.finish()
    }

    /** 不要这一段（撤回、返回键、换输入框、收起键盘）：编辑中的字撤掉。 */
    override fun onCancel() {
        val recognizer = recognizer ?: return
        recognizer.cancel()
        host.composeVoice("")
        end()
    }

    /** 外面要停（返回键、换输入框、键盘重建）：同撤回。 */
    fun cancel() = onCancel()

    override fun onPartial(text: String) {
        if (active) host.composeVoice(text)
    }

    override fun onFinal(text: String) {
        val then = after
        end()
        if (text.isBlank()) {
            host.composeVoice("")
        } else {
            host.commitVoice(text)
            if (then == After.ACTION) host.performVoiceAction()
        }
        if (then == After.HIDE) host.hideKeyboard()
    }

    override fun onError(message: String) {
        host.composeVoice("")
        end()
        host.toast(message)
    }

    override fun onLevel(level: Float) {
        panel?.addLevel(level)
    }

    private fun end() {
        recognizer = null
        panel = null
        after = After.NOTHING
        host.closeVoicePanel()
    }
}
