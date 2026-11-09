package app.qingjian.inputmethod.settings.page

import android.view.View
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.cloud.CloudConfig
import app.qingjian.inputmethod.cloud.SecretStore
import app.qingjian.inputmethod.core.Native
import app.qingjian.inputmethod.settings.Settings
import kotlin.concurrent.thread
import org.json.JSONObject

/**
 * 云端大模型二级页：接口地址、模型、密钥（Keystore 加密存），测试连接与清除密钥；
 * 下面是用到它的几处（云联想、补充译词），没配好时只给一行说明。
 */
class CloudSettingsActivity : PageActivity() {
    override val title = R.string.section_cloud

    /** 上次「测试连接」的结果（给用户看的一句话）；测试中是「测试中…」。 */
    private var testResult: String? = null

    override fun content(): List<View> {
        val cloud = CloudConfig.load(this)
        return listOf(
            ui.group(getString(R.string.section_service), serviceRows(cloud), getString(R.string.cloud_note)),
            ui.group(null, actionRows(cloud)),
            ui.group(getString(R.string.section_cloud_uses), useRows(cloud), getString(R.string.cloud_uses_note)),
        )
    }

    private fun serviceRows(cloud: CloudConfig) = listOf(
        ui.input(R.string.cloud_base_url, cloud.baseUrl, cloud.baseUrl, secret = false) {
            CloudConfig.set(this, CloudConfig.KEY_BASE_URL, it.ifEmpty { CloudConfig.DEFAULT_BASE_URL })
            testResult = null
            rebuild()
        },
        ui.input(R.string.cloud_model, cloud.model, cloud.model, secret = false) {
            CloudConfig.set(this, CloudConfig.KEY_MODEL, it.ifEmpty { CloudConfig.DEFAULT_MODEL })
            testResult = null
            rebuild()
        },
        ui.input(R.string.cloud_key, cloud.keyHint ?: getString(R.string.cloud_key_empty), "", secret = true) {
            if (it.isNotEmpty()) SecretStore.save(this, it)
            testResult = null
            rebuild()
        },
    )

    private fun actionRows(cloud: CloudConfig): List<View> {
        val rows = mutableListOf<View>(ui.action(R.string.cloud_test, testResult) { testConnection(cloud) })
        if (cloud.keyHint != null) {
            rows += ui.action(R.string.cloud_clear_key, null, color = ui.colors.danger) {
                SecretStore.clear(this)
                testResult = null
                rebuild()
            }
        }
        return rows
    }

    /** 没配好云端时开关摆着也不生效，换成说明。 */
    private fun useRows(cloud: CloudConfig): List<View> {
        if (!cloud.ready) {
            return listOf(
                ui.row(R.string.cloud_predict_title, R.string.gloss_fill_unavailable),
                ui.row(R.string.gloss_fill_title, R.string.gloss_fill_unavailable),
            )
        }
        val settings = Settings.load(this)
        val rows = mutableListOf<View>(
            ui.toggle(R.string.cloud_predict_title, R.string.cloud_predict_note, settings.cloudPredict) {
                prefs.edit().putBoolean(Settings.KEY_CLOUD_PREDICT, it).apply()
                rebuild()
            },
        )
        if (settings.cloudPredict) {
            rows += ui.toggle(R.string.cloud_wifi_only_title, R.string.cloud_wifi_only_note, settings.cloudWifiOnly) {
                prefs.edit().putBoolean(Settings.KEY_CLOUD_WIFI_ONLY, it).apply()
            }
        }
        rows += ui.toggle(R.string.gloss_fill_title, R.string.gloss_fill_note, settings.glossFill) {
            prefs.edit().putBoolean(Settings.KEY_GLOSS_FILL, it).apply()
        }
        return rows
    }

    /** 测试连接要等网络，放后台线程；回来再重画这一页。 */
    private fun testConnection(cloud: CloudConfig) {
        if (testResult == getString(R.string.cloud_testing)) return
        if (!cloud.ready) {
            testResult = getString(R.string.translate_no_cloud)
            rebuild()
            return
        }
        testResult = getString(R.string.cloud_testing)
        rebuild()
        val settings = cloud.toJson()
        thread(name = "qingjian-cloud-test") {
            val outcome = runCatching { JSONObject(Native.cloudTest(settings) ?: "{}") }.getOrNull()
            val message = outcome?.optString("message")?.takeIf { it.isNotEmpty() } ?: "测试失败"
            runOnUiThread {
                if (isDestroyed) return@runOnUiThread
                testResult = message
                rebuild()
            }
        }
    }
}
