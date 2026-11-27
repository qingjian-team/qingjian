//! 物理候选窗的无障碍节点及其位图坐标。

package app.qingjian.inputmethod.candidates.physical.virtual

import android.graphics.RectF

internal sealed interface VirtualItem {
    val virtualId: Int

    val rect: RectF
}
