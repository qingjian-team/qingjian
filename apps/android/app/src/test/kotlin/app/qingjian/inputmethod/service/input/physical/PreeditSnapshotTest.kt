//! 重启时清空、替换、保留组字与中间光标的内容核对。

package app.qingjian.inputmethod.service.input.physical

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PreeditSnapshotTest {
    @Test fun emptyOrReplacedEditorDoesNotResumeOldComposition() {
        assertNull(PreeditSnapshot.split("", "", listOf("ni", "ni'h")))
        assertNull(PreeditSnapshot.split("replacement", "", listOf("ni", "ni'h")))
    }

    @Test fun retainedCompositionMatchesAtEndAndAtInteriorCaret() {
        assertEquals(4 to 0, PreeditSnapshot.split("prefix ni'h", "", listOf("ni", "ni'h")))
        assertEquals(2 to 2, PreeditSnapshot.split("prefix ni", "'h suffix", listOf("ni", "ni'h")))
    }
}
