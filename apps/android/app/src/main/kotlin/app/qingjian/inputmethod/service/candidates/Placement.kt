//! 候选框避让输入行与屏幕边缘；坐标统一在宿主视图内。

package app.qingjian.inputmethod.service.candidates

object Placement {
    fun position(
        left: Int, top: Int, right: Int, bottom: Int,
        width: Int, height: Int, caretX: Float?, caretTop: Float?, caretBottom: Float?, gap: Int,
    ): Pair<Int, Int> {
        val maxX = (right - width).coerceAtLeast(left)
        val maxY = (bottom - height).coerceAtLeast(top)
        val x = (caretX?.toInt() ?: left).coerceIn(left, maxX)
        val y = if (caretTop == null || caretBottom == null) maxY else {
            val below = caretBottom.toInt() + gap
            val above = caretTop.toInt() - gap - height
            when {
                below + height <= bottom -> below
                above >= top -> above
                bottom - caretBottom >= caretTop - top -> below
                else -> above
            }.coerceIn(top, maxY)
        }
        return x to y
    }
}
