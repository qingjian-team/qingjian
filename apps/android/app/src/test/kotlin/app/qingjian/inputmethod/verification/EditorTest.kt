//! 文本验证码输入框可提示，密码与私密输入除外。

package app.qingjian.inputmethod.verification

import org.junit.Assert.assertTrue
import org.junit.Assert.assertFalse
import org.junit.Test

class EditorTest {
    @Test fun acceptsTextAndNumbersButNotPasswordPhoneUrlOrEmail() {
        assertTrue(acceptsVerificationCode(1, 0))
        assertTrue(acceptsVerificationCode(2, 0))
        assertTrue(acceptsVerificationCode(0x20001, 0))
        for (type in listOf(0, 3, 4, 0x81, 0x91, 0xe1, 0x12, 0x21, 0xd1, 0x11)) {
            assertFalse("inputType=$type", acceptsVerificationCode(type, 0))
        }
        assertFalse(acceptsVerificationCode(1, 0x1000000))
        assertFalse(acceptsVerificationCode(2, 0x1000000))
    }
}
