package app.qingjian.inputmethod.core

import android.content.Context
import java.io.File

/** 随包数据从 APK assets 解到私有目录：.qj 要 mmap，必须是真文件。装了新包才重解。 */
internal object BundledData {
    private const val ASSET_DIR = "data"

    /** 解出来的目录；App 页面只读里面的小表（等级表），没解过就当没有。 */
    fun dir(context: Context) = File(context.filesDir, "data")

    fun ensure(context: Context): File {
        val target = dir(context)
        val stamp = File(target, ".stamp")
        val version = context.packageManager
            .getPackageInfo(context.packageName, 0).lastUpdateTime.toString()
        if (stamp.isFile && stamp.readText() == version) return target

        target.deleteRecursively()
        copy(context, ASSET_DIR, target)
        stamp.writeText(version)
        return target
    }

    /** assets 里列得出子项的是目录（领域词库在 `dicts/` 下），递归拷。 */
    private fun copy(context: Context, asset: String, target: File) {
        target.mkdirs()
        for (name in context.assets.list(asset).orEmpty()) {
            val path = "$asset/$name"
            if (context.assets.list(path).orEmpty().isNotEmpty()) {
                copy(context, path, File(target, name))
                continue
            }
            context.assets.open(path).use { input ->
                File(target, name).outputStream().use { output -> input.copyTo(output) }
            }
        }
    }
}
