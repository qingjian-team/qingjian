//! 多来源验证码的短期内存；同一条用过后不因输入框重建而再次出现。

package app.qingjian.inputmethod.verification

object Inbox {
    private val entries = mutableMapOf<String, Code>()
    private val consumed = linkedMapOf<Pair<String, String>, Long>()
    private val consumedAutomatic = linkedMapOf<String, Long>()
    private var sequence = 0L
    var onChanged: (() -> Unit)? = null

    fun offer(source: String, id: String, code: String, masked: Boolean, expiresAt: Long) {
        val now = System.currentTimeMillis()
        consumed.entries.removeAll { it.value <= now }
        consumedAutomatic.entries.removeAll { it.value <= now }
        if (expiresAt <= now || consumed.containsKey(source to id)) return
        if (source != "clipboard" && consumedAutomatic.containsKey(code)) return
        val previous = entries[source]
        if (previous?.id == id && previous.code == code && previous.masked == masked) return
        entries[source] = Code(source, id, code, masked, expiresAt, ++sequence)
        onChanged?.invoke()
    }

    fun current(now: Long = System.currentTimeMillis()): Code? {
        entries.entries.removeAll { it.value.expiresAt <= now }
        return entries.values.maxByOrNull { it.sequence }
    }

    fun consume(item: Code) {
        // 同一短信常同时经短信广播和通知到达，只能上屏一次；用户重新复制仍可主动使用。
        for (entry in entries.values.filter { it.code == item.code }) {
            consumed[entry.source to entry.id] = entry.expiresAt
            entries.remove(entry.source)
        }
        consumedAutomatic[item.code] = minOf(item.expiresAt, System.currentTimeMillis() + 30_000L)
        while (consumed.size > 128) consumed.remove(consumed.keys.first())
        while (consumedAutomatic.size > 128) consumedAutomatic.remove(consumedAutomatic.keys.first())
        onChanged?.invoke()
    }

    fun clear(source: String) {
        if (entries.remove(source) != null) onChanged?.invoke()
    }

    fun remove(source: String, id: String) {
        if (entries[source]?.id == id) clear(source)
    }
}
