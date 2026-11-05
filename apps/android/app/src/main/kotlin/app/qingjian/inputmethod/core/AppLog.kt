package app.qingjian.inputmethod.core

import android.app.ActivityManager
import android.app.ApplicationExitInfo
import android.content.Context
import android.content.SharedPreferences
import android.os.Build
import android.util.Log
import java.io.File

/**
 * 应用日志：Kotlin 这边的事件与没接住的异常经 Rust 写进 `files/logs/`，和引擎的日志同一份文件，
 * 设置里「打包日志」交的就是这个目录。另外每次起来补记上次进程是怎么没的（Android 11 起系统才给）。
 * 缺省不记打的字；开了详细日志引擎的 debug 也进文件，见 `apps/android/core/src/jni/logging.rs`。
 */
object AppLog {
    private const val TAG = "qingjian"

    /** 只放日志自己的状态：放进设置那份会触发输入法重建键盘。 */
    private const val PREFS = "diagnostics"
    private const val KEY_VERBOSE = "verbose_log"
    private const val KEY_LAST_EXIT = "last_exit_seen"

    private var ready = false

    fun logDir(context: Context): File = File(context.filesDir, "logs")

    /** 应用进程一起来调一次（[app.qingjian.inputmethod.QingjianApp]）。 */
    fun init(context: Context) {
        if (ready) return
        ready = runCatching { Native.initLogging(logDir(context).absolutePath) }.isSuccess
        if (!ready) return
        Native.setVerboseLog(prefs(context).getBoolean(KEY_VERBOSE, false))
        val previous = Thread.getDefaultUncaughtExceptionHandler()
        Thread.setDefaultUncaughtExceptionHandler { thread, error ->
            error("uncaught exception on ${thread.name}: ${Log.getStackTraceString(error)}")
            previous?.uncaughtException(thread, error)
        }
        info("app start: ${Build.MANUFACTURER} ${Build.MODEL}, Android ${Build.VERSION.RELEASE} (SDK ${Build.VERSION.SDK_INT})")
        recordExitReasons(context)
    }

    fun verbose(context: Context): Boolean = prefs(context).getBoolean(KEY_VERBOSE, false)

    fun setVerbose(context: Context, on: Boolean) {
        prefs(context).edit().putBoolean(KEY_VERBOSE, on).apply()
        if (ready) Native.setVerboseLog(on)
        info("verbose log ${if (on) "on" else "off"}")
    }

    fun error(message: String) = write(0, Log.ERROR, message)

    fun warn(message: String) = write(1, Log.WARN, message)

    fun info(message: String) = write(2, Log.INFO, message)

    fun debug(message: String) = write(3, Log.DEBUG, message)

    /** 没装上 so 时退回 logcat。 */
    private fun write(level: Int, priority: Int, message: String) {
        if (ready) Native.log(level, message) else Log.println(priority, TAG, message)
    }

    /**
     * 上次之后进程退出过几回、各是什么原因（崩溃、ANR、被系统杀、低内存）：键盘「有时候弹不出来」多半要靠它。
     * 只记上回看过之后的新条目。
     */
    private fun recordExitReasons(context: Context) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.R) return
        val manager = context.getSystemService(ActivityManager::class.java) ?: return
        val reasons = runCatching { manager.getHistoricalProcessExitReasons(context.packageName, 0, 10) }
            .getOrNull() ?: return
        val seen = prefs(context).getLong(KEY_LAST_EXIT, 0)
        val fresh = reasons.filter { it.timestamp > seen }.sortedBy { it.timestamp }
        for (exit in fresh) {
            val line = "previous exit: ${reasonName(exit.reason)} at ${java.time.Instant.ofEpochMilli(exit.timestamp)}, " +
                "process ${exit.processName}, importance ${exit.importance}, pss ${exit.pss} kB, " +
                "description ${exit.description}"
            if (exit.reason == ApplicationExitInfo.REASON_CRASH ||
                exit.reason == ApplicationExitInfo.REASON_CRASH_NATIVE ||
                exit.reason == ApplicationExitInfo.REASON_ANR
            ) {
                warn(line)
            } else {
                info(line)
            }
        }
        fresh.lastOrNull()?.let { prefs(context).edit().putLong(KEY_LAST_EXIT, it.timestamp).apply() }
    }

    private fun reasonName(reason: Int): String = when (reason) {
        ApplicationExitInfo.REASON_ANR -> "anr"
        ApplicationExitInfo.REASON_CRASH -> "crash"
        ApplicationExitInfo.REASON_CRASH_NATIVE -> "native crash"
        ApplicationExitInfo.REASON_DEPENDENCY_DIED -> "dependency died"
        ApplicationExitInfo.REASON_EXCESSIVE_RESOURCE_USAGE -> "excessive resource usage"
        ApplicationExitInfo.REASON_EXIT_SELF -> "exit self"
        ApplicationExitInfo.REASON_INITIALIZATION_FAILURE -> "initialization failure"
        ApplicationExitInfo.REASON_LOW_MEMORY -> "low memory"
        ApplicationExitInfo.REASON_OTHER -> "other"
        ApplicationExitInfo.REASON_FREEZER -> "freezer"
        ApplicationExitInfo.REASON_PACKAGE_STATE_CHANGE -> "package state change"
        ApplicationExitInfo.REASON_PACKAGE_UPDATED -> "package updated"
        ApplicationExitInfo.REASON_PERMISSION_CHANGE -> "permission change"
        ApplicationExitInfo.REASON_SIGNALED -> "signaled"
        ApplicationExitInfo.REASON_USER_REQUESTED -> "user requested"
        ApplicationExitInfo.REASON_USER_STOPPED -> "user stopped"
        else -> "unknown ($reason)"
    }

    private fun prefs(context: Context): SharedPreferences =
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
}
