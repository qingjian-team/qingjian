package app.qingjian.inputmethod.settings.page

import android.app.AlertDialog
import android.content.Intent
import android.net.Uri
import android.view.View
import android.widget.Toast
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.core.Engine
import java.time.LocalDate
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream
import kotlin.concurrent.thread

/**
 * 高级页（PC 设置程序的「高级」改成手机的样子）：不露文件路径，只有「导出数据」与「清空学习数据」。
 * 导出用系统的「存到哪里」（SAF `ACTION_CREATE_DOCUMENT`）存一个 zip，打包哪些文件由 Rust 侧定；清空问两次。
 * PC 的配置文件编辑与输入日志开关不做。
 */
class AdvancedActivity : PageActivity() {
    override val title = R.string.page_advanced

    /** 正在写压缩包（后台线程），行上显示「导出中…」。 */
    private var exporting = false

    override fun content(): List<View> = listOf(
        ui.sectionTitle(R.string.section_data),
        ui.card(
            ui.action(R.string.export_title, getString(if (exporting) R.string.export_running else R.string.export_value)) {
                if (!exporting) pickTarget()
            },
            ui.note(R.string.export_note),
            ui.divider(),
            ui.action(R.string.clear_learning_title, null, DANGER) { chooseParts() },
            ui.note(R.string.clear_learning_note),
        ),
    )

    private fun pickTarget() {
        val intent = Intent(Intent.ACTION_CREATE_DOCUMENT)
            .addCategory(Intent.CATEGORY_OPENABLE)
            .setType("application/zip")
            .putExtra(Intent.EXTRA_TITLE, "qingjian-data-${LocalDate.now()}.zip")
        startActivityForResult(intent, CREATE_ZIP)
    }

    @Deprecated("Activity 的老接口，这页没有别的结果要收")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        val target = data?.data
        if (requestCode != CREATE_ZIP || resultCode != RESULT_OK || target == null) return
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
            .setTextColor(DANGER)
    }

    private companion object {
        const val CREATE_ZIP = 1

        /** 清空这类不能撤销的操作用红字。 */
        const val DANGER = 0xFFE5484D.toInt()
    }
}
