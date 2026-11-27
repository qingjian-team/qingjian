//! 核对重启后的编辑内容是否仍包含当前组字，避免把已清空的拼音接回来。

package app.qingjian.inputmethod.service.input.physical

object PreeditSnapshot {
    fun split(before: String, after: String, versions: List<String>): Pair<Int, Int>? {
        for (text in versions.filter(String::isNotEmpty).sortedByDescending(String::length)) {
            for (offset in 0..text.length) {
                if (before.endsWith(text.substring(0, offset)) && after.startsWith(text.substring(offset))) {
                    return offset to text.length - offset
                }
            }
        }
        return null
    }
}
