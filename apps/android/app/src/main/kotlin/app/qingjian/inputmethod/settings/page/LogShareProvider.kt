package app.qingjian.inputmethod.settings.page

import android.content.ContentProvider
import android.content.ContentValues
import android.content.Context
import android.database.Cursor
import android.database.MatrixCursor
import android.net.Uri
import android.os.ParcelFileDescriptor
import android.provider.OpenableColumns
import java.io.File
import java.io.FileNotFoundException

/**
 * 「打包日志 → 发送给…」把打好的 zip（`cache/log-share/`）只读地交给 QQ、微信、邮件这些应用。
 * 不导出，靠分享时的临时授权；没引 androidx，同 [app.qingjian.inputmethod.clipboard.ClipboardImageProvider] 自己写。
 */
class LogShareProvider : ContentProvider() {
    override fun onCreate() = true

    override fun openFile(uri: Uri, mode: String): ParcelFileDescriptor {
        if (mode != "r") throw FileNotFoundException("read only")
        return ParcelFileDescriptor.open(file(uri), ParcelFileDescriptor.MODE_READ_ONLY)
    }

    override fun getType(uri: Uri): String = MIME

    private fun file(uri: Uri): File {
        val name = uri.lastPathSegment ?: throw FileNotFoundException("no file name")
        val dir = shareDir(requireNotNull(context))
        val file = File(dir, name)
        // 只认这个目录里的文件名，挡住 ../ 一类
        if (file.parentFile?.canonicalPath != dir.canonicalPath || !file.isFile) throw FileNotFoundException(name)
        return file
    }

    /** 文件名与大小：微信、QQ 收文件时拿它显示名字，查不到就成了一串乱码名。 */
    override fun query(uri: Uri, projection: Array<out String>?, selection: String?, selectionArgs: Array<out String>?, sortOrder: String?): Cursor {
        val file = file(uri)
        val columns = projection?.filter { it in COLUMNS }?.toTypedArray() ?: COLUMNS
        return MatrixCursor(columns, 1).apply {
            addRow(columns.map { if (it == OpenableColumns.DISPLAY_NAME) file.name else file.length() })
        }
    }

    override fun insert(uri: Uri, values: ContentValues?): Uri? = null

    override fun update(uri: Uri, values: ContentValues?, selection: String?, selectionArgs: Array<out String>?) = 0

    override fun delete(uri: Uri, selection: String?, selectionArgs: Array<out String>?) = 0

    companion object {
        const val AUTHORITY = "${app.qingjian.inputmethod.BuildConfig.APPLICATION_ID}.logs"

        const val MIME = "application/zip"

        private val COLUMNS = arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE)

        fun shareDir(context: Context) = File(context.cacheDir, "log-share")

        fun uriOf(file: File): Uri = Uri.Builder().scheme("content").authority(AUTHORITY).appendPath(file.name).build()
    }
}
