package app.qingjian.inputmethod.voice

/** 没有能用的识别方式（「语音识别」没配好）时用：入口点下去提示「还不能用」。 */
object UnavailableRecognizer : VoiceRecognizer {
    override val available = false

    override val needsMicrophone = false

    override fun start(listener: VoiceRecognizer.Listener) = Unit

    override fun finish() = Unit

    override fun cancel() = Unit
}
