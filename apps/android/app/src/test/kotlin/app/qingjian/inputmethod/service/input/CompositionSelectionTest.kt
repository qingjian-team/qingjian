//! 物理光标回调不能结束拼音，同时保留触屏与移出组字的既有语义。

package app.qingjian.inputmethod.service.input

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class CompositionSelectionTest {
    @Test fun physicalCaretCanMoveToStartMiddleAndEndWithoutSettling() {
        for (cursor in 7..12) assertFalse(CompositionSelection.movedAway(true, cursor, cursor, 7, 12))
    }

    @Test fun leavingCompositionOrSelectingTextStillSettles() {
        assertTrue(CompositionSelection.movedAway(true, 6, 6, 7, 12))
        assertTrue(CompositionSelection.movedAway(true, 13, 13, 7, 12))
        assertTrue(CompositionSelection.movedAway(true, 8, 10, 7, 12))
        assertFalse(CompositionSelection.movedAway(true, 4, 4, -1, -1))
    }

    @Test fun touchKeepsItsExistingEndOnlyRule() {
        assertTrue(CompositionSelection.movedAway(false, 9, 9, 7, 12))
        assertFalse(CompositionSelection.movedAway(false, 12, 12, 7, 12))
    }
}
