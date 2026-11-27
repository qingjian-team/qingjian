//! 应用内点击拼音会同步引擎，输入法自己的光标回调不重入。

package app.qingjian.inputmethod.service.input.physical

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CaretTest {
    @Test fun ownCaretAndApplicationTapAreDistinct() {
        val caret = Caret()
        caret.expect("ni'hao", 3)
        assertNull(caret.changed("ni'hao", 13, 13, 10, 16))
        assertEquals(1, caret.changed("ni'hao", 11, 11, 10, 16))
        assertEquals(6, caret.changed("ni'hao", 16, 16, 10, 16))
        caret.expect("ni'hao", 1)
        assertNull(caret.changed("ni'hao", 11, 11, 10, 16))
    }

    @Test fun invalidOrSelectedRangesAreNotMappedAndUnicodeUsesCodePoints() {
        val caret = Caret()
        caret.expect("a😀b", 1)
        assertEquals(2, caret.changed("a😀b", 13, 13, 10, 14))
        assertNull(caret.changed("a😀b", 9, 9, 10, 14))
        assertNull(caret.changed("a😀b", 11, 12, 10, 14))
        assertNull(caret.changed("a😀b", 11, 11, -1, -1))
        assertNull(caret.changed("a😀b", 11, 11, 10, 15))
    }
}
