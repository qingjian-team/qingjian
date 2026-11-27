//! 光标锚定避让：下面、上翻、右边缘、小视口与无位置回退。

package app.qingjian.inputmethod.service.candidates

import org.junit.Assert.assertEquals
import org.junit.Test

class PlacementTest {
    @Test fun tracksCaretAndFlipsAboveNearBottom() {
        assertEquals(100 to 136, Placement.position(6, 6, 994, 794, 240, 280, 100f, 100f, 130f, 6))
        assertEquals(754 to 414, Placement.position(6, 6, 994, 794, 240, 280, 980f, 700f, 730f, 6))
    }
    @Test fun clampsOversizedWindowAndFallsBackWhenNoAnchor() {
        assertEquals(6 to 6, Placement.position(6, 6, 194, 194, 240, 280, 50f, 20f, 40f, 6))
        assertEquals(6 to 514, Placement.position(6, 6, 994, 794, 240, 280, null, null, null, 6))
    }
}
