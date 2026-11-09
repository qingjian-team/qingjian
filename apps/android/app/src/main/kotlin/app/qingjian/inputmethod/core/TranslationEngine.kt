package app.qingjian.inputmethod.core

import android.content.Context
import android.os.Handler
import android.os.Looper
import app.qingjian.inputmethod.cloud.CloudConfig
import app.qingjian.inputmethod.settings.Settings
import kotlin.concurrent.thread

/**
 * 「青简翻译」用的 Rust 句柄：只挂主词库与学习语言释义表的轻量 Engine（不装语言模型与学习数据），
 * 用来走 Core 的 `request_translation`（汉字为主译成学习语言、外文译成中文）。装配放后台线程，好了回调 [onReady]（失败为 false）。
 * 只在主线程用，用完 [close]。
 */
class TranslationEngine(context: Context, private val onReady: (Boolean) -> Unit) {
    private var handle = 0L
    private var closed = false

    init {
        val app = context.applicationContext
        val language = Settings.learningLanguage(app)
        val main = Handler(Looper.getMainLooper())
        thread(name = "qingjian-translation-load") {
            val dataDir = BundledData.ensure(app)
            val created = Native.createTranslation(dataDir.path, Engine.userDir(app).path, language)
            main.post {
                if (closed) {
                    Native.destroy(created)
                } else {
                    handle = created
                    onReady(created != 0L)
                }
            }
        }
    }

    /** 接上云端并发出这段文字；没装好、没配好或发不出返回 false。 */
    fun translate(config: CloudConfig, text: String): Boolean =
        handle != 0L && config.ready && Native.setCloud(handle, config.toJson()) && Native.translate(handle, text)

    /** 译文：null 是还没到，空串是模型没给出内容。 */
    fun poll(): String? = if (handle != 0L) Native.pollTranslation(handle) else null

    fun close() {
        closed = true
        if (handle != 0L) Native.destroy(handle)
        handle = 0
    }
}
