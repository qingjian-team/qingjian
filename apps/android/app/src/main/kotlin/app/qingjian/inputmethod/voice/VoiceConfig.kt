package app.qingjian.inputmethod.voice

import android.content.Context
import app.qingjian.inputmethod.cloud.SecretStore
import org.json.JSONObject

/**
 * 「语音识别」设置页的配置，与云端大模型（翻译、联想）那份分开存（`voice` 那份 prefs）：识别服务、计费方式、密钥。
 * 密钥与云端那份一样经 [SecretStore] 加密，只存密文；[toJson] 是交给 Rust 的那份（含解好的密钥），只在调 JNI 时临时拼。
 */
class VoiceConfig(val provider: String, val resourceId: String, private val apiKey: String?) {
    /** 选了服务且填了密钥才算配好。 */
    val ready get() = provider == PROVIDER_VOLCENGINE && !apiKey.isNullOrBlank()

    /** 设置页显示用：只露末尾 4 位。 */
    val keyHint: String? get() = apiKey?.takeIf { it.length > 4 }?.let { "····${it.takeLast(4)}" }

    fun toJson(): String = JSONObject()
        .put("provider", provider)
        .put("api_key", apiKey.orEmpty())
        .put("resource_id", resourceId)
        .toString()

    companion object {
        const val PROVIDER_NONE = "none"
        const val PROVIDER_VOLCENGINE = "volcengine"

        /** 豆包流式语音识别模型 2.0 的两种计费方式（与 Rust 侧 `VolcengineConfig` 的常数一致）。 */
        const val RESOURCE_HOURLY = "volc.seedasr.sauc.duration"
        const val RESOURCE_CONCURRENT = "volc.seedasr.sauc.concurrent"

        const val KEY_PROVIDER = "provider"
        const val KEY_RESOURCE = "resource_id"
        private const val KEY_CIPHER = "api_key_cipher"

        fun load(context: Context): VoiceConfig {
            val prefs = prefs(context)
            return VoiceConfig(
                prefs.getString(KEY_PROVIDER, null) ?: PROVIDER_NONE,
                prefs.getString(KEY_RESOURCE, null) ?: RESOURCE_HOURLY,
                prefs.getString(KEY_CIPHER, null)?.let(SecretStore::open),
            )
        }

        fun set(context: Context, key: String, value: String) {
            prefs(context).edit().putString(key, value).apply()
        }

        fun saveKey(context: Context, secret: String) {
            prefs(context).edit().putString(KEY_CIPHER, SecretStore.seal(secret.trim())).apply()
        }

        fun clearKey(context: Context) {
            prefs(context).edit().remove(KEY_CIPHER).apply()
        }

        fun prefs(context: Context) = context.getSharedPreferences("voice", Context.MODE_PRIVATE)
    }
}
