//! 修饰键单击识别的设备、组合与取消回归。

package app.qingjian.inputmethod.service.input.physical

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** 修饰键轻按不能把组合键、重复或旧设备抬起误判成语言切换。 */
class ModifierTapTest {
    @Test fun independentPressMatchesOnlyItsOwnRelease() {
        val tap = ModifierTap()
        tap.begin(1, 59, 100)
        assertTrue(tap.release(1, 59, 100, false))
        assertFalse(tap.release(1, 59, 100, false))
    }

    @Test fun chordRepeatAndDeviceChangesCancelTap() {
        val tap = ModifierTap()
        tap.begin(1, 59, 100)
        tap.cancel()
        assertFalse(tap.release(1, 59, 100, false))
        tap.begin(1, 59, 100)
        assertFalse(tap.release(2, 59, 100, false))
        tap.begin(1, 59, 100)
        assertFalse(tap.release(1, 59, 100, true))
        tap.begin(1, 59, 100)
        assertFalse(tap.release(1, 59, 101, false))
    }
}
