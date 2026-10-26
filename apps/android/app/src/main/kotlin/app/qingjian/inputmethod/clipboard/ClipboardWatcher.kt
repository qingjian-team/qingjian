package app.qingjian.inputmethod.clipboard

import android.content.ClipDescription
import android.content.ClipboardManager
import android.content.Context
import android.os.Build
import android.os.Handler
import android.os.Looper
import app.qingjian.inputmethod.core.store.ClipboardStore
import app.qingjian.inputmethod.settings.Settings
import kotlin.concurrent.thread

/**
 * 盯着系统剪贴板，复制了文字或图片就记进剪贴板历史。默认输入法读剪贴板不要权限（安卓 10 起只有它和前台应用能读）。
 * 标了敏感的（密码管理器复制的密码，安卓 13 的 `EXTRA_IS_SENSITIVE`）不记；设置里关了「记录剪贴板」也不记。
 * 图片读字节放后台线程，太大的（超过 [MAX_IMAGE_BYTES]）不记。
 */
class ClipboardWatcher(private val context: Context) {
    private val manager = context.getSystemService(ClipboardManager::class.java)
    private val main = Handler(Looper.getMainLooper())
    private val listener = ClipboardManager.OnPrimaryClipChangedListener { capture() }

    fun start() {
        manager?.addPrimaryClipChangedListener(listener)
    }

    fun stop() {
        manager?.removePrimaryClipChangedListener(listener)
    }

    /** 读一次当前剪贴板记下来；键盘弹出时也调（监听在输入法没起来时收不到）。 */
    fun capture() {
        val settings = Settings.load(context)
        if (!settings.recordClipboard) return
        val clip = runCatching { manager?.primaryClip }.getOrNull() ?: return
        if (clip.itemCount == 0 || sensitive(clip.description)) return
        val item = clip.getItemAt(0)
        val uri = item.uri
        val mime = uri?.let { runCatching { context.contentResolver.getType(it) }.getOrNull() }
        if (uri != null && mime?.startsWith("image/") == true) {
            thread(name = "qingjian-clipboard-image") {
                val bytes = runCatching {
                    context.contentResolver.openInputStream(uri)?.use { input ->
                        val buffer = input.readNBytesCompat(MAX_IMAGE_BYTES + 1)
                        buffer.takeIf { it.size <= MAX_IMAGE_BYTES }
                    }
                }.getOrNull() ?: return@thread
                ClipboardStore.addImage(context, bytes, ClipboardImageProvider.extensionOf(mime), settings.clipboardLimit)
                main.post { onChanged?.invoke() }
            }
            return
        }
        val text = item.text?.toString() ?: return
        ClipboardStore.add(context, text, settings.clipboardLimit)
        onChanged?.invoke()
    }

    /** 记进去了一条（剪贴板面板开着时重读）。 */
    var onChanged: (() -> Unit)? = null

    private fun sensitive(description: ClipDescription): Boolean =
        Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            description.extras?.getBoolean(ClipDescription.EXTRA_IS_SENSITIVE) == true

    private fun java.io.InputStream.readNBytesCompat(limit: Int): ByteArray {
        val out = java.io.ByteArrayOutputStream()
        val buffer = ByteArray(64 * 1024)
        while (out.size() < limit) {
            val read = read(buffer, 0, minOf(buffer.size, limit - out.size()))
            if (read < 0) break
            out.write(buffer, 0, read)
        }
        return out.toByteArray()
    }

    private companion object {
        const val MAX_IMAGE_BYTES = 10 * 1024 * 1024
    }
}
