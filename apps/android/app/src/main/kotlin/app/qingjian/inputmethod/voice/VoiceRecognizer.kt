package app.qingjian.inputmethod.voice

/**
 * 一种语音识别方式。入口（长按空格、工具栏话筒、全面屏底栏的语音键）都经 [VoiceInput] 挑一个实现来用，
 * 换识别方式只换实现：系统 `SpeechRecognizer`、调系统语音输入法、Rust 侧的云端或离线模型（[EngineRecognizer]）。
 * 识别出的文字一律交回 [VoiceInput]，经 Core 上屏。主线程调，回调也在主线程。
 */
interface VoiceRecognizer {
    /** 现在能不能用（装了、配好了）；不能用时入口只提示「还不能用」。 */
    val available: Boolean

    /** 要录音权限：自己录音的要，交给系统语音输入法的不要。 */
    val needsMicrophone: Boolean

    /** 开始听，结果经 [listener] 回来。 */
    fun start(listener: Listener)

    /** 说完了：停止录音，等最终结果（[Listener.onFinal] 或 [Listener.onError]）。 */
    fun finish()

    /** 丢掉这一段，之后不再回调。 */
    fun cancel()

    interface Listener {
        /** 到目前为止听出来的整段文字（不是增量）。 */
        fun onPartial(text: String)

        /** 最终结果；之后不再回调。 */
        fun onFinal(text: String)

        /** 出错，给用户看的一句话；之后不再回调。 */
        fun onError(message: String)

        /** 音量，0 到 1，画话筒的波动用。 */
        fun onLevel(level: Float)
    }
}
