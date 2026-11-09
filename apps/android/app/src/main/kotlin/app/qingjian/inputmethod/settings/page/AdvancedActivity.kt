package app.qingjian.inputmethod.settings.page

import android.app.AlertDialog
import android.content.ClipData
import android.content.Intent
import android.net.Uri
import android.view.View
import android.widget.Toast
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.core.AppLog
import app.qingjian.inputmethod.core.Engine
import java.time.LocalDate
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream
import kotlin.concurrent.thread

/**
 * 高级页（PC 设置程序的「高级」改成手机的样子）：不露文件路径，有「导出数据」「清空学习数据」与日志（打包、详细日志）。
 * 导出与打包日志都用系统的「存到哪里」（SAF `ACTION_CREATE_DOCUMENT`）存一个 zip，数据打包哪些文件由 Rust 侧定，
 * 日志见 [LogPack]；清空问两次。
 * PC 的配置文件编辑与输入日志开关不做。
 */
class AdvancedActivity : PageActivity() {
    override val title = R.string.page_advanced

    /** 正在写压缩包（后台线程），行上显示「导出中…」。 */
    private var exporting = false

    override fun content(): List<View> = listOf(
        ui.group(
            null,
            ui.action(R.string.export_title, getString(if (exporting) R.string.export_running else R.string.export_value)) {
                if (!exporting) pickTarget()
            },
            footer = R.string.export_note,
        ),
        ui.group(
            null,
            ui.action(R.string.clear_learning_title, null, ui.colors.danger) { chooseParts() },
            footer = R.string.clear_learning_note,
        ),
        ui.group(
            R.string.logs_section,
            ui.action(R.string.logs_export_title, null) { chooseLogTarget() },
            ui.toggle(R.string.logs_verbose_title, R.string.logs_verbose_note, AppLog.verbose(this)) {
                AppLog.setVerbose(this, it)
            },
            footer = R.string.logs_export_note,
        ),
    )

    private fun pickTarget(request: Int = CREATE_ZIP) {
        val name = if (request == PACK_LOGS) "qingjian-logs" else "qingjian-data"
        val intent = Intent(Intent.ACTION_CREATE_DOCUMENT)
            .addCategory(Intent.CATEGORY_OPENABLE)
            .setType("application/zip")
            .putExtra(Intent.EXTRA_TITLE, "$name-${LocalDate.now()}.zip")
        startActivityForResult(intent, request)
    }

    @Deprecated("Activity 的老接口，这页没有别的结果要收")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        val target = data?.data
        if (resultCode != RESULT_OK || target == null) return
        if (requestCode == PACK_LOGS) {
            packLogs(target)
            return
        }
        if (requestCode != CREATE_ZIP) return
        Engine.flushAll()
        val files = Engine.learningFiles(this)
        exporting = true
        rebuild()
        thread(name = "qingjian-export") {
            val message = runCatching { writeZip(target, files) }
                .fold({ getString(R.string.export_done, it) }, { getString(R.string.export_failed) })
            runOnUiThread {
                if (isDestroyed) return@runOnUiThread
                exporting = false
                rebuild()
                Toast.makeText(this, message, Toast.LENGTH_SHORT).show()
            }
        }
    }

    /** 发出去排第一：手机上存下来的文件多数人找不到，反馈又多在 QQ 群里。 */
    private fun chooseLogTarget() {
        AlertDialog.Builder(this)
            .setTitle(R.string.logs_export_title)
            .setItems(arrayOf(getString(R.string.logs_send), getString(R.string.logs_save))) { _, which ->
                if (which == 0) shareLogs() else pickTarget(PACK_LOGS)
            }
            .show()
    }

    private fun shareLogs() {
        thread(name = "qingjian-logs") {
            val file = runCatching { LogPack.forSharing(this, "qingjian-logs-${LocalDate.now()}.zip") }
                .onFailure { AppLog.error("log export failed: ${android.util.Log.getStackTraceString(it)}") }
            runOnUiThread {
                if (isDestroyed) return@runOnUiThread
                val zip = file.getOrNull()
                when {
                    file.isFailure -> Toast.makeText(this, R.string.export_failed, Toast.LENGTH_SHORT).show()
                    zip == null -> Toast.makeText(this, R.string.logs_export_empty, Toast.LENGTH_SHORT).show()
                    else -> {
                        val uri = LogShareProvider.uriOf(zip)
                        val send = Intent(Intent.ACTION_SEND)
                            .setType(LogShareProvider.MIME)
                            .putExtra(Intent.EXTRA_STREAM, uri)
                            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                        send.clipData = ClipData.newRawUri(zip.name, uri)
                        startActivity(Intent.createChooser(send, getString(R.string.logs_send)))
                    }
                }
            }
        }
    }

    private fun packLogs(target: Uri) {
        thread(name = "qingjian-logs") {
            val message = runCatching { LogPack.save(this, target) }.fold(
                { if (it == 0) getString(R.string.logs_export_empty) else getString(R.string.logs_export_done, it) },
                {
                    AppLog.error("log export failed: ${android.util.Log.getStackTraceString(it)}")
                    getString(R.string.export_failed)
                },
            )
            runOnUiThread {
                if (!isDestroyed) Toast.makeText(this, message, Toast.LENGTH_SHORT).show()
            }
        }
    }

    /** 文件平铺在压缩包根上；读不到的跳过。返回写进去几个。 */
    private fun writeZip(target: Uri, files: List<Pair<String, java.io.File>>): Int {
        val output = contentResolver.openOutputStream(target, "wt") ?: error("打不开导出位置")
        var written = 0
        ZipOutputStream(output).use { zip ->
            for ((name, file) in files) {
                val bytes = runCatching { file.readBytes() }.getOrNull() ?: continue
                zip.putNextEntry(ZipEntry(name).apply { time = file.lastModified() })
                zip.write(bytes)
                zip.closeEntry()
                written++
            }
        }
        return written
    }

    /** 第一次：勾清哪几样（缺省全勾）；第二次：确认不能恢复。 */
    private fun chooseParts() {
        val checked = booleanArrayOf(true, true)
        AlertDialog.Builder(this)
            .setTitle(R.string.clear_learning_title)
            .setMultiChoiceItems(
                arrayOf(getString(R.string.clear_habits), getString(R.string.clear_vocabulary)), checked,
            ) { _, which, on -> checked[which] = on }
            .setPositiveButton(R.string.clear_next) { _, _ ->
                if (checked.any { it }) confirm(checked[0], checked[1])
            }
            .setNegativeButton(android.R.string.cancel, null)
            .show()
    }

    private fun confirm(habits: Boolean, vocabulary: Boolean) {
        val parts = listOfNotNull(
            getString(R.string.clear_habits_short).takeIf { habits },
            getString(R.string.clear_vocabulary_short).takeIf { vocabulary },
        ).joinToString("和")
        AlertDialog.Builder(this)
            .setMessage(getString(R.string.clear_confirm, parts))
            .setPositiveButton(R.string.clear_do) { _, _ ->
                val removed = Engine.clearLearning(this, habits, vocabulary)
                val message = if (removed < 0) R.string.clear_failed else R.string.clear_done
                Toast.makeText(this, message, Toast.LENGTH_SHORT).show()
            }
            .setNegativeButton(android.R.string.cancel, null)
            .show()
            .getButton(AlertDialog.BUTTON_POSITIVE)
            .setTextColor(ui.colors.danger)
    }

    private companion object {
        const val CREATE_ZIP = 1
        const val PACK_LOGS = 2
    }
}
