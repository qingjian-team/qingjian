package app.qingjian.inputmethod.settings.page

import android.app.AlertDialog
import android.content.Intent
import android.net.Uri
import android.provider.OpenableColumns
import android.view.Gravity
import android.view.View
import android.widget.Toast
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.core.BundledData
import app.qingjian.inputmethod.core.InputOptions
import app.qingjian.inputmethod.core.Native
import app.qingjian.inputmethod.settings.Settings
import java.io.File
import kotlin.concurrent.thread
import org.json.JSONArray
import org.json.JSONObject

/**
 * 词库页（PC 设置程序的「词库」）：随包领域词库勾选开关，导入的词库可开关、删除；「导入词库」用系统文件选择器，
 * 选中的文件拷进缓存目录后交给 Rust 转成 `.qj` 放进学习数据目录。开关存设置，键盘重建时随选项交给会话重挂。
 */
class DictionariesActivity : PageActivity() {
    override val title = R.string.page_dictionaries

    private class Entry(val id: String, val name: String, val entries: Int, val license: String)

    private var domains: List<Entry> = emptyList()
    private var imported: List<Entry> = emptyList()

    /** 正在导入（后台线程），按钮显示「导入中…」。 */
    private var importing = false

    override fun onResume() {
        super.onResume()
        load(JSONObject())
    }

    override fun content(): List<View> {
        val options = Settings.load(this).inputOptions
        return listOf(
            ui.sectionTitle(R.string.section_domains),
            domainsCard(options),
            spaced(ui.sectionTitle(R.string.section_imported), 20),
            importedCard(options),
        )
    }

    private fun domainsCard(options: InputOptions): View {
        val rows = mutableListOf<View>()
        domains.forEachIndexed { i, entry ->
            if (i > 0) rows += ui.divider()
            rows += ui.toggle(entry.name, getString(R.string.dictionary_entries, entry.entries), entry.id in options.domains, null) {
                val current = Settings.load(this).inputOptions.domains
                val picked = if (it) current + entry.id else current - entry.id
                prefs.edit().putStringSet(Settings.KEY_DOMAINS, picked).apply()
            }
        }
        rows += ui.note(R.string.domains_note)
        return ui.card(*rows.toTypedArray())
    }

    private fun importedCard(options: InputOptions): View {
        val rows = mutableListOf<View>()
        if (imported.isEmpty()) rows += ui.row(getString(R.string.dictionary_none), null)
        imported.forEachIndexed { i, entry ->
            if (i > 0) rows += ui.divider()
            val note = if (entry.license.isEmpty()) {
                getString(R.string.dictionary_entries, entry.entries)
            } else {
                getString(R.string.dictionary_entries_license, entry.entries, entry.license)
            }
            val delete = ui.text(getString(R.string.dictionary_delete), 14f, ui.colors.secondary).apply {
                gravity = Gravity.CENTER
                setPadding(ui.dp(12), ui.dp(10), ui.dp(12), ui.dp(10))
                setOnClickListener { confirmDelete(entry) }
            }
            rows += ui.toggle(entry.name, note, entry.id !in options.disabledDictionaries, delete) {
                val current = Settings.load(this).inputOptions.disabledDictionaries
                val disabled = if (it) current - entry.id else current + entry.id
                prefs.edit().putStringSet(Settings.KEY_DISABLED_DICTIONARIES, disabled).apply()
            }
        }
        rows += ui.divider()
        rows += ui.action(if (importing) R.string.dictionary_importing else R.string.dictionary_import, null, ui.colors.brand) {
            if (!importing) pickFiles()
        }
        rows += ui.note(R.string.imported_note)
        return ui.card(*rows.toTypedArray())
    }

    private fun confirmDelete(entry: Entry) {
        AlertDialog.Builder(this)
            .setMessage(getString(R.string.dictionary_delete_confirm, entry.name))
            .setPositiveButton(R.string.dictionary_delete) { _, _ -> load(JSONObject().put("remove", entry.id), changed = true) }
            .setNegativeButton(android.R.string.cancel, null)
            .show()
    }

    /** TSV 与 Rime 的 MIME 类型各家文件管理器给得不一样，不按类型筛。 */
    private fun pickFiles() {
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT)
            .addCategory(Intent.CATEGORY_OPENABLE)
            .setType("*/*")
            .putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true)
        startActivityForResult(intent, PICK_FILES)
    }

    @Deprecated("Activity 的老接口，这页没有别的结果要收")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode != PICK_FILES || resultCode != RESULT_OK || data == null) return
        val uris = data.clipData?.let { clip -> List(clip.itemCount) { clip.getItemAt(it).uri } } ?: listOfNotNull(data.data)
        if (uris.isEmpty()) return
        importing = true
        rebuild()
        thread(name = "qingjian-dictionary-import") {
            val messages = uris.map(::importOne)
            runOnUiThread {
                if (isDestroyed) return@runOnUiThread
                importing = false
                touchStamp()
                load(JSONObject())
                Toast.makeText(this, messages.joinToString("\n"), Toast.LENGTH_LONG).show()
            }
        }
    }

    /** 拷到缓存目录（保留原文件名，Rust 按它起词库的 id），导入完删掉拷贝；返回给用户看的一句。 */
    private fun importOne(uri: Uri): String {
        val name = displayName(uri)?.replace('/', '_') ?: "dictionary.tsv"
        val dir = File(cacheDir, "import").apply { mkdirs() }
        val copy = File(dir, name)
        return try {
            contentResolver.openInputStream(uri)?.use { input -> copy.outputStream().use(input::copyTo) }
                ?: return "导入失败：读不到「$name」"
            val page = Native.dictionaries(dataDir(), userDir(), JSONObject().put("import", copy.path).toString())
            page?.let { JSONObject(it).optString("message") }?.takeIf { it.isNotEmpty() } ?: "导入失败"
        } catch (error: Exception) {
            "导入失败：读不到「$name」"
        } finally {
            copy.delete()
        }
    }

    private fun displayName(uri: Uri): String? =
        contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
            if (cursor.moveToFirst()) cursor.getString(0) else null
        }

    /** 交请求、读回清单再重画；[changed] 时词库文件变了，改设置里的戳让开着的键盘重挂。 */
    private fun load(request: JSONObject, changed: Boolean = false) {
        val page = Native.dictionaries(dataDir(), userDir(), request.toString())?.let(::JSONObject)
        domains = page?.optJSONArray("domains").entries()
        imported = page?.optJSONArray("imported").entries()
        if (changed) {
            touchStamp()
            page?.optString("message")?.takeIf { it.isNotEmpty() }?.let { Toast.makeText(this, it, Toast.LENGTH_SHORT).show() }
        }
        rebuild()
    }

    private fun touchStamp() {
        prefs.edit().putLong(Settings.KEY_DICTIONARIES_STAMP, System.currentTimeMillis()).apply()
    }

    private fun JSONArray?.entries(): List<Entry> {
        if (this == null) return emptyList()
        return List(length()) { i ->
            val item = getJSONObject(i)
            Entry(item.getString("id"), item.getString("name"), item.getInt("entries"), item.optString("license"))
        }
    }

    private fun dataDir() = BundledData.dir(this).path

    private fun userDir() = File(filesDir, "user").path

    private companion object {
        const val PICK_FILES = 1
    }
}
