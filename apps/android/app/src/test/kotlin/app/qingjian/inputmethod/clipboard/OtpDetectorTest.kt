//! 验证码识别与输入范围回归。

package app.qingjian.inputmethod.clipboard

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class OtpDetectorTest {
    @Test fun findsStandaloneCode() {
        assertEquals("482913", OtpDetector.detect("482913"))
    }

    @Test fun findsCodeWithChineseContext() {
        assertEquals("901244", OtpDetector.detect("验证码：901244，请在五分钟内使用"))
    }

    @Test fun rejectsLongNumbers() {
        assertNull(OtpDetector.detect("13800138000"))
    }

    @Test fun rejectsUnrelatedText() {
        assertNull(OtpDetector.detect("订单号 482913"))
    }

    @Test fun findsCodeBeforeEnglishContext() {
        assertEquals("482913", OtpDetector.detect("482913 is your verification code"))
    }
    @Test fun acceptsFormattedTotpAndPreservesLeadingZero() {
        assertEquals("012345", OtpDetector.detect("012 345"))
        assertEquals("12345678", OtpDetector.detect("1234-5678"))
        assertEquals("012345", OtpDetector.detect("TOTP: 012345"))
    }

    @Test fun smsAndNotificationsRequireContext() {
        assertNull(OtpDetector.detect("123456", contextualOnly = true))
        assertNull(OtpDetector.detect("Order 123456 delivered", contextualOnly = true))
        assertEquals("012345", OtpDetector.detect("Your verification code is 012345", contextualOnly = true))
        assertNull(OtpDetector.detect("手机号 13800138000，验证码见下一条", contextualOnly = true))
    }
}
