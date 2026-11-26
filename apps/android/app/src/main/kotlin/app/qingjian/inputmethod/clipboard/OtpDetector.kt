//! 在本机提取短验证码；短信和通知要求明确验证码上下文。

package app.qingjian.inputmethod.clipboard

object OtpDetector {
    private const val LABEL = "(?:验证码|动态码|校验码|验证代码|認証コード|verification\\s*code|security\\s*code|one[- ]time\\s*(?:password|code)|\\botp\\b|\\btotp\\b|\\bpasscode\\b)"
    private const val CODE = "(?<![A-Za-z0-9])([0-9]{3}[ \\u00a0-][0-9]{3}|[0-9]{4}[ \\u00a0-][0-9]{4}|[0-9]{4,8})(?![A-Za-z0-9])"
    private val before = Regex("$LABEL[^0-9\\n]{0,24}$CODE", RegexOption.IGNORE_CASE)
    private val after = Regex("$CODE[^0-9\\n]{0,24}$LABEL", RegexOption.IGNORE_CASE)
    private val standalone = Regex("^$CODE$")

    fun detect(text: String, contextualOnly: Boolean = false): String? {
        val value = text.trim().take(4096)
        val match = before.find(value) ?: after.find(value)
            ?: if (!contextualOnly && text.length <= 32) standalone.matchEntire(value) else null
        return match?.groupValues?.get(1)?.filter(Char::isDigit)
    }
}
