//! 验证码过期、消费去重以及不同来源互不清空的回归。

package app.qingjian.inputmethod.verification

import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertNotNull
import org.junit.Test

class InboxTest {
    @After fun clear() {
        Inbox.onChanged = null
        listOf("test-clip", "test-sms", "test-notification", "clipboard").forEach(Inbox::clear)
    }

    @Test fun consumedClipboardDoesNotReappearOnRefocus() {
        val expiry = System.currentTimeMillis() + 60_000
        Inbox.offer("clipboard", "consumed", "123456", true, expiry)
        Inbox.consume(Inbox.current()!!)
        Inbox.offer("clipboard", "consumed", "123456", true, expiry)
        assertNull(Inbox.current())
        Inbox.offer("clipboard", "fresh-copy", "123456", true, expiry)
        assertEquals("fresh-copy", Inbox.current()?.id)
    }

    @Test fun expiredEntriesNeverShowAndReReadingDoesNotExtendExpiry() {
        val now = System.currentTimeMillis()
        Inbox.offer("test-clip", "old", "238715", false, now - 1)
        assertNull(Inbox.current())
        Inbox.offer("test-clip", "short", "238715", false, now + 1000)
        Inbox.offer("test-clip", "short", "238715", false, now + 90_000)
        assertNull(Inbox.current(now + 1001))
    }

    @Test fun unchangedClipboardCannotReplaceNewSms() {
        val expiry = System.currentTimeMillis() + 60_000
        Inbox.offer("test-clip", "clip", "543218", false, expiry)
        Inbox.offer("test-sms", "sms", "789012", false, expiry)
        Inbox.offer("test-clip", "clip", "543218", false, expiry)
        assertEquals("sms", Inbox.current()?.id)
        Inbox.clear("test-clip")
        assertEquals("sms", Inbox.current()?.id)
        Inbox.remove("test-sms", "unrelated")
        assertNotNull(Inbox.current())
    }

    @Test fun consumesSameSmsAndNotificationCodeIncludingDelayedNotification() {
        val expiry = System.currentTimeMillis() + 60_000
        Inbox.offer("test-sms", "one-sms", "692841", true, expiry)
        Inbox.offer("test-notification", "same-sms", "692841", true, expiry)
        Inbox.consume(Inbox.current()!!)
        assertNull(Inbox.current())
        Inbox.offer("test-notification", "late-update", "692841", true, expiry)
        assertNull(Inbox.current())
        Inbox.offer("clipboard", "manual-copy", "692841", true, expiry)
        assertNotNull(Inbox.current())
    }
}
