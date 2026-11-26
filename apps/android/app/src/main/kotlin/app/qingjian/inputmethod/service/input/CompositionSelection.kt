//! 区分拼音内的物理光标移动与离开组字的选区变化。

package app.qingjian.inputmethod.service.input

object CompositionSelection {
    fun movedAway(physical: Boolean, start: Int, end: Int, composingStart: Int, composingEnd: Int): Boolean {
        if (composingStart < 0 || composingEnd < composingStart) return false
        // 物理键盘的 ←→/Home/End 会把折叠光标放进组字内；触屏保留原先仅末尾不落下的约定。
        if (physical && start == end && start in composingStart..composingEnd) return false
        return start != composingEnd || end != composingEnd
    }
}
