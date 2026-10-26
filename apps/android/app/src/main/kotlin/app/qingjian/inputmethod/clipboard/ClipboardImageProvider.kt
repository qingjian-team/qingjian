package app.qingjian.inputmethod.clipboard

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
 * 把剪贴板历史里的图片（`files/user/clipboard-images/`）只读地交给别的应用：发图片走 `commitContent`、
 * 放回系统剪贴板走 `ClipData.newUri`，对方都拿这里的 content:// 地址读。不导出，靠临时授权。
 * 没引 androidx，所以不用 FileProvider，只开这一个目录、只读。
 */
class ClipboardImageProvider : ContentProvider() {
    override fun onCreate() = true

    override fun openFile(uri: Uri, mode: String): ParcelFileDescriptor {
        if (mode != "r") throw FileNotFoundException("只读")
        return ParcelFileDescriptor.open(file(uri), ParcelFileDescriptor.MODE_READ_ONLY)
    }

    override fun getType(uri: Uri): String = mimeOf(uri.lastPathSegment.orEmpty())

    private fun file(uri: Uri): File {
        val name = uri.lastPathSegment ?: throw FileNotFoundException("没有文件名")
        val dir = imageDir(requireNotNull(context))
        val file = File(dir, name)
        // 只认这个目录里的文件名，挡住 ../ 一类
        if (file.parentFile?.canonicalPath != dir.canonicalPath || !file.isFile) throw FileNotFoundException(name)
        return file
    }

    /** 文件名、大小、类型。有的应用收图前先查这几列，查不到（返回 null）就把地址当文字插进输入框。 */
    override fun query(uri: Uri, projection: Array<out String>?, selection: String?, selectionArgs: Array<out String>?, sortOrder: String?): Cursor {
        val file = file(uri)
        val columns = projection?.filter { it in COLUMNS }?.toTypedArray() ?: COLUMNS
        return MatrixCursor(columns, 1).apply {
            addRow(
                columns.map {
                    when (it) {
                        OpenableColumns.DISPLAY_NAME -> file.name
                        OpenableColumns.SIZE -> file.length()
                        else -> mimeOf(file.name)
                    }
                },
            )
        }
    }

    override fun insert(uri: Uri, values: ContentValues?): Uri? = null

    override fun update(uri: Uri, values: ContentValues?, selection: String?, selectionArgs: Array<out String>?) = 0

    override fun delete(uri: Uri, selection: String?, selectionArgs: Array<out String>?) = 0

    companion object {
        const val AUTHORITY = "app.qingjian.inputmethod.clipboard"

        private val COLUMNS = arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE, "mime_type")

        fun imageDir(context: Context) = File(context.filesDir, "user/clipboard-images")

        /** 图片文件（完整路径）对应的 content:// 地址。 */
        fun uriOf(path: String): Uri = Uri.Builder().scheme("content").authority(AUTHORITY).appendPath(File(path).name).build()

        fun mimeOf(name: String): String = when (name.substringAfterLast('.').lowercase()) {
            "jpg", "jpeg" -> "image/jpeg"
            "gif" -> "image/gif"
            "webp" -> "image/webp"
            else -> "image/png"
        }

        /** MIME 对应的扩展名（存图时用）。 */
        fun extensionOf(mime: String): String = when (mime.lowercase()) {
            "image/jpeg", "image/jpg" -> "jpg"
            "image/gif" -> "gif"
            "image/webp" -> "webp"
            else -> "png"
        }
    }
}
