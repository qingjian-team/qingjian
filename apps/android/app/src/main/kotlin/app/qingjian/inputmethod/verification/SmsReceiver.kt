//! 只处理用户授权后新到达的短信；短信正文不落盘、不进入剪贴板。

package app.qingjian.inputmethod.verification

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.provider.Telephony
import app.qingjian.inputmethod.clipboard.OtpDetector
import java.security.MessageDigest

class SmsReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Telephony.Sms.Intents.SMS_RECEIVED_ACTION || !Sources.sms(context)) return
        val messages = runCatching { Telephony.Sms.Intents.getMessagesFromIntent(intent) }.getOrNull() ?: return
        if (messages.isEmpty()) return
        val body = messages.joinToString("") { it.messageBody.orEmpty() }
        val code = OtpDetector.detect(body, contextualOnly = true) ?: return
        // 多段短信合并后去重；标识只在内存中使用，不暴露号码或正文。
        val identity = messages.first().timestampMillis.toString() + ":" +
            messages.first().originatingAddress.orEmpty() + ":" + body
        val id = MessageDigest.getInstance("SHA-256").digest(identity.toByteArray())
            .joinToString("") { "%02x".format(it) }
        Inbox.offer("sms", id, code, masked = true, expiresAt = System.currentTimeMillis() + 300_000L)
    }
}
