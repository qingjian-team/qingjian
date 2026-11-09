package app.qingjian.inputmethod.clipboard.sync

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.os.Build
import android.os.Handler
import android.os.Looper
import app.qingjian.inputmethod.core.Native
import app.qingjian.inputmethod.core.store.ClipboardStore
import app.qingjian.inputmethod.settings.Settings
import kotlin.concurrent.thread
import org.json.JSONObject

/**
 * 局域网同步在安卓这边的管家：按设置开关 Rust 的同步服务和系统的局域网发现、取事件转给界面。
 * 只在进程活着时同步（键盘服务或设置页），不用常驻通知；键盘弹出时补一轮，系统杀过进程也接得上。
 */
object ClipboardSync {
    const val KEY_ENABLED = "clipboard_sync"
    const val KEY_NEON = "clipboard_sync_neon"
    const val KEY_DEVICE_NAME = "clipboard_device_name"

    private const val POLL_MILLIS = 500

    /** 界面的监听：霓虹灯和设备页。主线程回调。 */
    interface Listener {
        fun onSyncEvent(type: String, event: JSONObject)
    }

    private val main = Handler(Looper.getMainLooper())
    private val listeners = LinkedHashSet<Listener>()
    private var running = false
    private var poller: Thread? = null
    private var discovery: NsdDiscovery? = null

    /** 刚由同步放上系统剪贴板的文字：监听到它不再记一遍，不然时刻变新又传回去，两边来回传。 */
    @Volatile
    var lastReceived: String? = null
        private set

    val isRunning get() = running

    fun enabled(context: Context) = Settings.preferences(context).getBoolean(KEY_ENABLED, false)

    fun neonEnabled(context: Context) = Settings.preferences(context).getBoolean(KEY_NEON, true)

    fun addListener(listener: Listener) {
        listeners.add(listener)
    }

    fun removeListener(listener: Listener) {
        listeners.remove(listener)
    }

    fun deviceName(context: Context): String {
        val saved = Settings.preferences(context).getString(KEY_DEVICE_NAME, "").orEmpty().trim()
        if (saved.isNotEmpty()) return saved
        val chinese = context.resources.configuration.locales[0].language == "zh"
        return runCatching { Native.clipboardSyncDefaultName(ClipboardStore.userDir(context), chinese) }.getOrNull().orEmpty()
    }

    /** 设置说开就开，说关就关；已经是那个状态就什么也不做。 */
    fun apply(context: Context) {
        val app = context.applicationContext
        if (enabled(app)) {
            if (!running) start(app)
        } else if (running) {
            stop()
        }
    }

    /** 改了设备名、条数上限：重开一次。 */
    fun restart(context: Context) {
        val app = context.applicationContext
        stop()
        if (enabled(app)) start(app)
    }

    private fun start(context: Context) {
        val limit = Settings.load(context).clipboardLimit
        val started = runCatching {
            Native.clipboardSyncStart(ClipboardStore.userDir(context), deviceName(context), Build.MODEL.orEmpty(), limit)
        }.getOrDefault(false)
        if (!started) return
        running = true
        val info = runCatching { Native.clipboardSyncInfo() }.getOrNull()?.let(::JSONObject)
        if (info != null) {
            discovery = NsdDiscovery(context, info.getString("id")).apply {
                start(deviceName(context), Build.MODEL.orEmpty(), info.getInt("port"))
            }
        }
        poller = thread(name = "qingjian-clipboard-sync", isDaemon = true) { poll(context) }
    }

    private fun stop() {
        running = false
        poller = null
        runCatching { Native.clipboardSyncStop() }
        discovery?.stop()
        discovery = null
    }

    /** 本机内容变了。 */
    fun changed() {
        if (running) runCatching { Native.clipboardSyncChanged() }
    }

    private fun poll(context: Context) {
        val self = Thread.currentThread()
        while (running && poller === self) {
            val json = runCatching { Native.clipboardSyncPoll(POLL_MILLIS) }.getOrNull() ?: continue
            val event = runCatching { JSONObject(json) }.getOrNull() ?: continue
            val type = event.optString("type")
            if (type == "finished" && !event.isNull("text")) putOnClipboard(context, event.getString("text"))
            main.post { listeners.toList().forEach { it.onSyncEvent(type, event) } }
        }
    }

    /** 输入法在后台也可以写系统剪贴板；读才受限。 */
    private fun putOnClipboard(context: Context, text: String) {
        lastReceived = text
        main.post {
            runCatching {
                context.getSystemService(ClipboardManager::class.java)
                    ?.setPrimaryClip(ClipData.newPlainText("Qingjian", text))
            }
        }
    }
}
