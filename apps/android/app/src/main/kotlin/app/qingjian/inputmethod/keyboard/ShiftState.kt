package app.qingjian.inputmethod.keyboard

/** Shift 的三种状态：单击亮一次（出一个大写字母后熄掉），双击锁定（一直大写，再点解锁）。 */
enum class ShiftState {
    OFF,
    ONCE,
    LOCKED;

    val on: Boolean get() = this != OFF
}
