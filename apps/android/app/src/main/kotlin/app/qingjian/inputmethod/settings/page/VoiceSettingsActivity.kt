package app.qingjian.inputmethod.settings.page

import android.view.View
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.core.Native
import app.qingjian.inputmethod.voice.VoiceConfig
import kotlin.concurrent.thread
import org.json.JSONObject

/** 语音识别二级页：识别服务、计费方式、API Key（Keystore 加密存），测试连接与清除密钥。与键盘设置里的「云端大模型」分开存。 */
class VoiceSettingsActivity : PageActivity() {
    override val title = R.string.page_voice

    /** 上次「测试连接」的结果；测试中是「测试中…」。 */
    private var testResult: String? = null

    override fun content(): List<View> {
        val config = VoiceConfig.load(this)
        val providers = listOf(
            VoiceConfig.PROVIDER_NONE to getString(R.string.voice_provider_none),
            VoiceConfig.PROVIDER_VOLCENGINE to getString(R.string.voice_provider_volcengine),
        )
        val provider = providers.indexOfFirst { it.first == config.provider }.coerceAtLeast(0)
        val rows = mutableListOf<View>(
            ui.picker(R.string.voice_provider, providers[provider].second, providers.map { it.second }, provider) {
                VoiceConfig.set(this, VoiceConfig.KEY_PROVIDER, providers[it].first)
                testResult = null
                rebuild()
            },
        )
        if (config.provider == VoiceConfig.PROVIDER_VOLCENGINE) rows += volcengineRows(config)
        else rows += ui.note(R.string.voice_none_note)
        return listOf(ui.sectionTitle(R.string.section_voice_service), ui.card(*rows.toTypedArray()))
    }

    private fun volcengineRows(config: VoiceConfig): List<View> {
        val resources = listOf(
            VoiceConfig.RESOURCE_HOURLY to getString(R.string.voice_resource_hourly),
            VoiceConfig.RESOURCE_CONCURRENT to getString(R.string.voice_resource_concurrent),
        )
        val resource = resources.indexOfFirst { it.first == config.resourceId }.coerceAtLeast(0)
        val rows = mutableListOf<View>(
            ui.divider(),
            ui.picker(R.string.voice_resource, resources[resource].second, resources.map { it.second }, resource) {
                VoiceConfig.set(this, VoiceConfig.KEY_RESOURCE, resources[it].first)
                testResult = null
                rebuild()
            },
            ui.divider(),
            ui.input(R.string.voice_key, config.keyHint ?: getString(R.string.cloud_key_empty), "", secret = true) {
                if (it.isNotEmpty()) VoiceConfig.saveKey(this, it)
                testResult = null
                rebuild()
            },
            ui.divider(),
            ui.action(R.string.cloud_test, testResult) { testConnection(config) },
        )
        if (config.keyHint != null) {
            rows += ui.divider()
            rows += ui.action(R.string.cloud_clear_key, null) {
                VoiceConfig.clearKey(this)
                testResult = null
                rebuild()
            }
        }
        rows += ui.note(R.string.voice_note)
        return rows
    }

    /** 测试连接要等网络（识别半秒静音），放后台线程；回来再重画这一页。 */
    private fun testConnection(config: VoiceConfig) {
        if (testResult == getString(R.string.cloud_testing)) return
        if (!config.ready) {
            testResult = getString(R.string.voice_unavailable)
            rebuild()
            return
        }
        testResult = getString(R.string.cloud_testing)
        rebuild()
        val settings = config.toJson()
        thread(name = "qingjian-voice-test") {
            val outcome = runCatching { JSONObject(Native.voiceTest(settings) ?: "{}") }.getOrNull()
            val message = outcome?.optString("message")?.takeIf { it.isNotEmpty() } ?: "测试失败"
            runOnUiThread {
                if (isDestroyed) return@runOnUiThread
                testResult = message
                rebuild()
            }
        }
    }
}
