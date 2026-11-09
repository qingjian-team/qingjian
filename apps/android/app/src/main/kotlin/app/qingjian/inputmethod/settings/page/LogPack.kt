package app.qingjian.inputmethod.settings.page

import android.content.Context
import android.net.Uri
import android.app.usage.UsageStatsManager
import android.os.Build
import android.os.PowerManager
import android.provider.Settings as AndroidSettings
import android.view.inputmethod.InputMethodManager
import app.qingjian.inputmethod.core.AppLog
import app.qingjian.inputmethod.settings.Settings
import java.io.File
import java.io.OutputStream
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream

/**
 * 「打包日志」：日志目录里的文件，加一份设置清单与机型信息，平铺进一个 zip，反馈问题时附上（同 PC 的「打包到桌面」）；
 * 发给别的应用走 [forSharing] + [LogShareProvider]，存到手机走 [save]。
 * 云端与语音的密钥在另一份加密存储里，不进包；设置里名字像密钥的项也抹掉，防将来有人放错地方。
 */
object LogPack {
    /** 存到用户选的位置（SAF），返回写进去几个文件；一个日志都没有时不写，返回 0。 */
    fun save(context: Context, target: Uri): Int {
        if (logs(context).isEmpty()) return 0
        val output = context.contentResolver.openOutputStream(target, "wt") ?: error("cannot open export target")
        return write(context, output)
    }

    /** 打到 [LogShareProvider] 的目录里给分享用（只留这一份）；没有日志返回 null。 */
    fun forSharing(context: Context, name: String): File? {
        if (logs(context).isEmpty()) return null
        val dir = LogShareProvider.shareDir(context)
        dir.deleteRecursively()
        dir.mkdirs()
        val file = File(dir, name)
        write(context, file.outputStream())
        return file
    }

    private fun logs(context: Context): List<File> =
        AppLog.logDir(context).listFiles { file -> file.isFile }?.sortedBy { it.name }.orEmpty()

    /** 文字的两份先算好再开压缩包：算到一半出错不留下半个包。写完关掉 [output]。 */
    private fun write(context: Context, output: OutputStream): Int {
        val logs = logs(context)
        val texts = listOf("settings.txt" to settings(context), "device.txt" to device(context))
        ZipOutputStream(output).use { zip ->
            for (file in logs) {
                val bytes = runCatching { file.readBytes() }.getOrNull() ?: continue
                zip.putNextEntry(ZipEntry(file.name).apply { time = file.lastModified() })
                zip.write(bytes)
                zip.closeEntry()
            }
            for ((name, text) in texts) {
                zip.putNextEntry(ZipEntry(name))
                zip.write(text.toByteArray())
                zip.closeEntry()
            }
        }
        AppLog.warn("user exported logs (${logs.size} files)")
        return logs.size + texts.size
    }

    private fun settings(context: Context): String =
        Settings.preferences(context).all.toSortedMap().entries.joinToString("\n") { (key, value) ->
            if (SECRET.containsMatchIn(key)) "$key = <removed>" else "$key = $value"
        } + "\n"

    /**
     * 机型与系统，外加事后补不回来、又常是原因的几样：省电限制与待机分组（进程被杀、键盘弹不出）、
     * 当前与已启用的输入法、实体键盘状态、随包数据在不在（没候选）、剩余空间。
     */
    private fun device(context: Context): String {
        val config = context.resources.configuration
        val resolver = context.contentResolver
        val lines = listOf(
            line("app") {
                val info = context.packageManager.getPackageInfo(context.packageName, 0)
                "${context.packageName} ${info.versionName.orEmpty()} (${info.longVersionCodeCompat()})"
            },
            line("device") { "${Build.MANUFACTURER} ${Build.MODEL} (${Build.DEVICE}), abi ${Build.SUPPORTED_ABIS.joinToString()}" },
            line("android") { "${Build.VERSION.RELEASE} (SDK ${Build.VERSION.SDK_INT}), build ${Build.DISPLAY}" },
            line("locale") { config.locales.toLanguageTags() },
            line("screen") {
                val metrics = context.resources.displayMetrics
                "${metrics.widthPixels}x${metrics.heightPixels} @ ${metrics.densityDpi} dpi, font scale ${config.fontScale}, " +
                    "orientation ${config.orientation}"
            },
            line("keyboard") { "${config.keyboard}, hardKeyboardHidden ${config.hardKeyboardHidden}" },
            line("ignoring battery optimizations") {
                context.getSystemService(PowerManager::class.java).isIgnoringBatteryOptimizations(context.packageName).toString()
            },
            line("standby bucket") {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
                    context.getSystemService(UsageStatsManager::class.java).appStandbyBucket.toString()
                } else {
                    "n/a"
                }
            },
            line("default ime") {
                AndroidSettings.Secure.getString(resolver, AndroidSettings.Secure.DEFAULT_INPUT_METHOD).toString()
            },
            line("enabled imes") {
                context.getSystemService(InputMethodManager::class.java).enabledInputMethodList.joinToString { it.id }
            },
            line("free space") { "${context.filesDir.usableSpace / 1_048_576} MB" },
            line("verbose log") { AppLog.verbose(context).toString() },
        )
        val data = runCatching { dataFiles(File(context.filesDir, "data")) }
            .getOrElse { "  unknown (${it.javaClass.simpleName})\n" }
        return lines.joinToString("\n") + "\ndata files:\n" + data
    }

    /** 一项取不到（系统不让读、服务不在）只写 unknown 和异常名，不让整份打包失败。 */
    private fun line(label: String, value: () -> String): String =
        "$label " + runCatching(value).getOrElse { "unknown (${it.javaClass.simpleName}: ${it.message})" }

    /** 随包数据解出来的文件与大小：缺了、大小为 0 都会没候选。 */
    private fun dataFiles(dir: File): String =
        dir.walkTopDown().filter { it.isFile }.sortedBy { it.path }.joinToString("") {
            "  ${it.relativeTo(dir)} ${it.length()}\n"
        }.ifEmpty { "  (none)\n" }

    @Suppress("DEPRECATION")
    private fun android.content.pm.PackageInfo.longVersionCodeCompat(): Long =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) longVersionCode else versionCode.toLong()

    /** `api_key`、`apiKey`、`token` 这类；`keyboard_height` 不算。 */
    private val SECRET = Regex("(^|_)(api_?key|token|secret|password)", RegexOption.IGNORE_CASE)
}
