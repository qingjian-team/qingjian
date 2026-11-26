//! Shift字符从按下起重复有效，中途改变修饰键才停止重复。

package app.qingjian.inputmethod.service.input.physical

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class PressesTest {
    @Test fun heldShiftCharacterRepeatsUntilModifiersChange() {
        val presses = Presses()
        presses.down(1, 29, 1, true)
        presses.down(1, 29, 1, false)
        assertFalse(presses.changed(1, 29, 1))
        assertTrue(presses.changed(1, 29, 0))
        presses.down(1, 29, 0, false)
        assertTrue(presses.changed(1, 29, 0))
        presses.up(1, 29)
        assertFalse(presses.changed(1, 29, 0))
    }

    @Test fun newPressAndDeviceResetReplaceOldState() {
        val presses = Presses()
        presses.down(1, 29, 1, true)
        presses.down(1, 29, 0, true)
        assertFalse(presses.changed(1, 29, 0))
        assertFalse(presses.changed(2, 29, 1))
        presses.clear()
        assertFalse(presses.changed(1, 29, 1))
    }
}
