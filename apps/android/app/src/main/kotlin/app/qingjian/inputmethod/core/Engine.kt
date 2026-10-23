package app.qingjian.inputmethod.core

import android.content.Context
import android.graphics.Bitmap
import android.os.Handler
import android.os.Looper
import app.qingjian.inputmethod.cloud.CloudConfig
import app.qingjian.inputmethod.core.frame.Frame
import app.qingjian.inputmethod.core.strip.StripImage
import app.qingjian.inputmethod.voice.VoiceConfig
import java.io.File
import java.nio.ByteBuffer
import kotlin.concurrent.thread
import org.json.JSONObject

/**
 * Rust Engine 的句柄。解随包数据和装配都慢（首次要解几十 MB），放后台线程；
 * 没装好之前 [ready] 为 false，壳把按键原样交给应用。
 */
class Engine(context: Context, private val onReady: () -> Unit) {
    private val app = context.applicationContext
    private var handle = 0L
    private var closed = false
    private val density = context.resources.displayMetrics.density

    /** 交给 Rust 的云端配置的摘要；0 是没接（新装配的引擎也是没接）。 */
    private var cloudSignature = 0

    /** 交给 Rust 的补充译词配置的摘要；0 是关着（新装配的引擎也是关着）。 */
    private var glossSignature = 0

    /** 交给 Rust 的语音识别配置的摘要；0 是没配（新装配的引擎也是）。 */
    private var voiceSignature = 0

    /** 第几次装配；卸下或重装时加一，晚到的旧装配结果直接销毁。 */
    private var generation = 0

    /** 候选条的像素中转与位图，只增不减地复用：每次敲键都画一帧，不能每次新开几 MB。 */
    private var stripBuffer: ByteBuffer? = null
    private var stripBitmap: Bitmap? = null

    /** 候选条的主题与字号档；引擎（重新）装好时照这份交过去。 */
    private var appearance: Pair<String, Float>? = null

    val ready: Boolean get() = handle != 0L

    init {
        live += this
        load()
    }

    private fun load() {
        val main = Handler(Looper.getMainLooper())
        val loading = ++generation
        thread(name = "qingjian-engine-load") {
            val dataDir = BundledData.ensure(app)
            val created = Native.create(dataDir.path, userDir(app).path)
            main.post {
                if (closed || loading != generation) {
                    Native.destroy(created)
                } else {
                    handle = created
                    glossSignature = 0
                    voiceSignature = 0
                    cloudSignature = 0
                    cloudConnected = false
                    predicting = false
                    appearance?.let { (theme, scale) -> if (ready) Native.setAppearance(handle, theme, scale) }
                    if (ready) onReady()
                }
            }
        }
    }

    /** 落盘并销毁 Rust 侧，装配到一半的作废；之后 [load] 重来。 */
    private fun unload() {
        generation++
        if (ready) Native.destroy(handle)
        handle = 0
    }

    fun push(codePoint: Int): Frame = Frame.parse(Native.push(handle, codePoint))

    fun backspace(): Frame = Frame.parse(Native.backspace(handle))

    /** 滑完一笔：解出的字母进组句，同一个个点出来。 */
    fun swipe(input: String): Frame = Frame.parse(Native.swipe(handle, input))

    fun decodeSwipe(input: String): String = if (ready) Native.decodeSwipe(handle, input).orEmpty() else ""

    fun select(index: Int): Frame = Frame.parse(Native.select(handle, index))

    fun selectTranslation(index: Int, sense: Int): Frame =
        Frame.parse(Native.selectTranslation(handle, index, sense))

    fun forget(index: Int): Frame = Frame.parse(Native.forget(handle, index))

    fun commitRaw(): Frame = Frame.parse(Native.commitRaw(handle))

    fun clear(): Frame = Frame.parse(Native.clear(handle))

    fun setNineKey(on: Boolean): Frame = Frame.parse(Native.setNineKey(handle, on))

    fun pin(alternative: String): Frame = Frame.parse(Native.pin(handle, alternative))

    fun setEnglishMode(on: Boolean): Frame = Frame.parse(Native.setEnglishMode(handle, on))

    fun setZhuyin(on: Boolean): Frame = Frame.parse(Native.setZhuyin(handle, on))

    fun setShuangpin(scheme: String, rawPreedit: Boolean): Frame = Frame.parse(Native.setShuangpin(handle, scheme, rawPreedit))

    fun setWubi(on: Boolean, mixed: Boolean): Frame = Frame.parse(Native.setWubi(handle, on, mixed))

    fun setOptions(options: InputOptions): Frame = Frame.parse(Native.setOptions(handle, options.toJson()))

    /** 中文标点键：Core 按设置转全角或原样；引擎没装好原样。 */
    fun punctuate(c: Char): String = (if (ready) Native.punctuate(handle, c) else null) ?: c.toString()

    /** 拆词；引擎没装好时一字一块。 */
    fun textPieces(text: String): List<String> {
        val json = if (ready) Native.textPieces(handle, text) else null
        json ?: return text.map(Char::toString)
        val array = org.json.JSONArray(json)
        return List(array.length(), array::getString)
    }

    fun reloadPhrases(userDir: String) {
        if (ready) Native.reloadPhrases(handle, userDir)
    }

    /**
     * 按配置接上云端（[predict] 时云联想与整句补全一起开），null 或没配好就断开；返回能不能翻译。
     * 同一份配置不重交（Rust 侧每接一次都另起后台线程），只记摘要，不留密钥明文。
     */
    fun setCloud(config: CloudConfig?, predict: Boolean): Boolean {
        if (!ready) return false
        val json = config?.takeIf { it.ready }?.toJson(predict)
        val signature = json?.hashCode() ?: 0
        if (signature != cloudSignature) {
            cloudSignature = signature
            cloudConnected = Native.setCloud(handle, json)
            predicting = cloudConnected && predict
        }
        return cloudConnected
    }

    /** 云端接着（能翻译）。 */
    var cloudConnected = false
        private set

    /** 云联想开着：组句时壳要轮询结果（[pollPrediction]）。 */
    var predicting = false
        private set

    /** 这段组句的光标前后文，云联想用。 */
    fun setSurrounding(before: String, after: String) {
        if (predicting) Native.setSurrounding(handle, before, after)
    }

    /** 云联想的结果插进候选后的一帧；没有新结果为 null。 */
    fun pollPrediction(): Frame? =
        if (predicting) Native.pollPrediction(handle)?.let(Frame::parse) else null

    /**
     * 释义兜底开或关（[config] 为 null 是关）。同一份配置不重交：每交一次 Rust 侧都另起一条后台线程。
     * 记的是配置的摘要，不留密钥明文。
     */
    fun setGlossFill(config: CloudConfig?) {
        val json = config?.takeIf { it.ready }?.toJson()
        val signature = json?.hashCode() ?: 0
        if (!ready || signature == glossSignature) return
        glossSignature = signature
        Native.setGlossFill(handle, json)
    }

    fun translate(text: String): Boolean = ready && Native.translate(handle, text)

    fun pollTranslation(): String? = if (ready) Native.pollTranslation(handle) else null

    fun setMetered(metered: Boolean) {
        if (ready) Native.setMetered(handle, metered)
    }

    fun setPrivate(private: Boolean) {
        if (ready) Native.setPrivate(handle, private)
    }

    fun cancelTranslation() {
        if (ready) Native.cancelTranslation(handle)
    }

    fun setJapanese(on: Boolean): Frame = Frame.parse(Native.setJapanese(handle, on))

    fun setTraditional(on: Boolean): Frame = Frame.parse(Native.setTraditional(handle, on))

    fun space(): Frame = Frame.parse(Native.space(handle))

    /** 光标前是「用户名@」时，帧的候选是常见邮箱域名，选中上屏剩下的那截；别的情况是空帧。 */
    fun complete(beforeCursor: String): Frame = Frame.parse(Native.complete(handle, beforeCursor))

    /** 算式里按了 [c]：要交给 [push] 的字符（× ÷ 换成 Core 认的 * /）；不是算式字符为 null。按 Core 的规则，不要句柄。 */
    fun expressionChar(c: Char): Char? = Native.expressionChar(c).takeIf { it != '\u0000' }

    /** 光标前最后一个字是 [c] 时可能是一道算式的结尾（数字、右括号、等号），才值得 [complete]。 */
    fun mayEndCalculation(c: Char): Boolean = Native.mayEndCalculation(c)

    /**
     * 画最新一帧的候选条。返回的位图下次调用时会被改写（复用），只在这一帧里用；没装好或画不出来为 null。
     */
    fun renderStrip(highlighted: Int, dark: Boolean, visibleWidth: Int): StripImage? {
        if (!ready) return null
        val meta = Native.renderStrip(handle, highlighted, density, dark, visibleWidth)?.let(::JSONObject) ?: return null
        val width = meta.getInt("width")
        val height = meta.getInt("height")
        if (width <= 0 || height <= 0) return null
        val bytes = width * height * 4
        val buffer = stripBuffer?.takeIf { it.capacity() >= bytes }
            ?: ByteBuffer.allocateDirect(bytes).also { stripBuffer = it }
        if (!Native.copyStrip(handle, buffer)) return null
        buffer.position(0).limit(bytes)
        val bitmap = stripBitmap?.takeIf { it.allocationByteCount >= bytes }
            ?.apply { if (this.width != width || this.height != height) reconfigure(width, height, Bitmap.Config.ARGB_8888) }
            ?: Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888).also { stripBitmap = it }
        bitmap.copyPixelsFromBuffer(buffer)
        buffer.clear()
        return StripImage.parse(meta, bitmap)
    }

    /** 候选条换主题与字号档（[textScale] 是候选字与译文一起的倍数）；没装好时记着，装好再交。 */
    fun setAppearance(theme: String, textScale: Float) {
        appearance = theme to textScale
        if (ready) Native.setAppearance(handle, theme, textScale)
    }

    /** 识别出的一段文字上屏：Core 转繁体、记输入历史与统计。 */
    fun commitVoice(text: String): Frame = Frame.parse(Native.commitVoice(handle, text))

    /**
     * 按「语音识别」设置换识别方，null 或没配好是不用。同一份配置不重交，只记摘要，不留密钥明文。
     */
    fun setVoice(config: VoiceConfig?) {
        val json = config?.takeIf { it.ready }?.toJson()
        val signature = json?.hashCode() ?: 0
        if (!ready || signature == voiceSignature) return
        voiceSignature = signature
        Native.setVoice(handle, json)
    }

    /** Rust 侧有能用的识别方（云端、离线模型）：「语音识别」里配好了。 */
    val voiceAvailable: Boolean get() = ready && Native.voiceAvailable(handle)

    fun voiceStart(sampleRate: Int) {
        if (ready) Native.voiceStart(handle, sampleRate)
    }

    fun voiceFeed(samples: ShortArray) {
        if (ready) Native.voiceFeed(handle, samples)
    }

    fun voiceFinish() {
        if (ready) Native.voiceFinish(handle)
    }

    fun voiceCancel() {
        if (ready) Native.voiceCancel(handle)
    }

    /** 识别的一件事：（partial / final / error，文字）；没有为 null。 */
    fun pollVoice(): Pair<String, String>? {
        val json = (if (ready) Native.pollVoice(handle) else null)?.let(::JSONObject) ?: return null
        return json.getString("kind") to json.getString("text")
    }

    fun reset() {
        if (ready) Native.reset(handle)
    }

    fun close() {
        closed = true
        live -= this
        if (ready) Native.destroy(handle)
        handle = 0
    }

    companion object {
        /** 进程里开着的引擎（输入法服务那一个；App 与输入法同进程）。只在主线程用。 */
        private val live = mutableSetOf<Engine>()

        fun userDir(context: Context) = File(context.filesDir, "user").apply { mkdirs() }

        /** 导出前把开着的引擎里还没落盘的学习数据写下（组句一并丢掉，导出时用户在 App 里）。主线程调。 */
        fun flushAll() = live.forEach(Engine::reset)

        /**
         * 清空学习数据，返回删掉的文件数（-1 是出错）。引擎内存里的学习数据下次落盘会把删掉的写回去，
         * 所以先落盘卸下开着的引擎、再删、再重新装配（装好之前按键原样上屏）。主线程调。
         */
        fun clearLearning(context: Context, habits: Boolean, vocabulary: Boolean): Int {
            live.forEach(Engine::unload)
            val removed = runCatching { Native.clearLearning(userDir(context).path, habits, vocabulary) }.getOrDefault(-1)
            live.forEach(Engine::load)
            return removed
        }

        /** 导出数据要打包的文件：(压缩包里的名字, 文件)。 */
        fun learningFiles(context: Context): List<Pair<String, File>> {
            val json = runCatching { Native.learningFiles(userDir(context).path) }.getOrNull() ?: return emptyList()
            val array = org.json.JSONArray(json)
            return List(array.length()) { i ->
                array.getJSONObject(i).let { it.getString("name") to File(it.getString("path")) }
            }
        }
    }
}
