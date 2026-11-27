//! 桌面候选主题的翻页命中区域。

package app.qingjian.inputmethod.core.strip

import android.graphics.RectF

/** 桌面主题的翻页箭头，坐标为位图像素。 */
data class StripPage(
    val step: Int,

    val rect: RectF,
)
