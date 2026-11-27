//! 记录一次独立修饰键轻按，组合与取消事件使它作废。

package app.qingjian.inputmethod.service.input.physical

/** 一个修饰键独自按下又抬起才算单击；重复、其他键、取消或设备改变都会取消。 */
class ModifierTap {
    private var device = -1

    private var key = -1

    private var down = 0L

    fun begin(deviceId: Int, keyCode: Int, downTime: Long) {
        device = deviceId
        key = keyCode
        down = downTime
    }

    fun release(deviceId: Int, keyCode: Int, downTime: Long, canceled: Boolean): Boolean {
        val matched = !canceled && device == deviceId && key == keyCode && down == downTime
        cancel()
        return matched
    }

    fun cancel() {
        device = -1
        key = -1
        down = 0L
    }
}
