package app.qingjian.inputmethod.cloud

import android.content.Context
import org.json.JSONObject

/**
 * 云端大模型的配置（OpenAI 兼容接口，快捷翻译、释义兜底与云联想用）：地址与模型是明文设置，密钥在 [SecretStore]。
 * [toJson] 是交给 Rust 的那份（含解好的密钥），只在调 JNI 时临时拼，不留存。
 */
class CloudConfig(val baseUrl: String, val model: String, private val apiKey: String?) {
    /** 三样都有才算配好。 */
    val ready get() = baseUrl.isNotBlank() && model.isNotBlank() && !apiKey.isNullOrBlank()

    /** 设置页显示用：只露末尾 4 位。 */
    val keyHint: String? get() = apiKey?.takeIf { it.length > 4 }?.let { "····${it.takeLast(4)}" }

    /** [predict] 时 Rust 侧连云联想与整句补全一起开，否则只做显式的翻译。 */
    fun toJson(predict: Boolean = false): String = JSONObject()
        .put("base_url", baseUrl)
        .put("model", model)
        .put("api_key", apiKey.orEmpty())
        .put("predict", predict)
        .toString()

    companion object {
        /** 与 PC 的 `PredictConfig` 缺省一致。 */
        const val DEFAULT_BASE_URL = "https://api.deepseek.com"
        const val DEFAULT_MODEL = "deepseek-flash"

        const val KEY_BASE_URL = "base_url"
        const val KEY_MODEL = "model"

        fun load(context: Context): CloudConfig {
            val prefs = SecretStore.prefs(context)
            return CloudConfig(
                prefs.getString(KEY_BASE_URL, null) ?: DEFAULT_BASE_URL,
                prefs.getString(KEY_MODEL, null) ?: DEFAULT_MODEL,
                SecretStore.load(context),
            )
        }

        fun set(context: Context, key: String, value: String) {
            SecretStore.prefs(context).edit().putString(key, value.trim()).apply()
        }
    }
}
