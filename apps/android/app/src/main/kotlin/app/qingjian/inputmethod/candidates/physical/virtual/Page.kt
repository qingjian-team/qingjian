//! 物理候选窗的翻页无障碍节点。

package app.qingjian.inputmethod.candidates.physical.virtual

import android.graphics.RectF

internal data class Page(
    override val virtualId: Int,

    val step: Int,

    override val rect: RectF,
) : VirtualItem
