//! 物理候选窗的候选词无障碍节点。

package app.qingjian.inputmethod.candidates.physical.virtual

import android.graphics.RectF

internal data class CandidateItem(
    override val virtualId: Int,

    val index: Int,

    val text: String,

    override val rect: RectF,
) : VirtualItem
