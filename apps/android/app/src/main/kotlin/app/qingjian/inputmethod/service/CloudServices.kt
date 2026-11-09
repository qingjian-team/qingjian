package app.qingjian.inputmethod.service

import android.content.Context
import android.content.SharedPreferences
import app.qingjian.inputmethod.cloud.CloudConfig
import app.qingjian.inputmethod.cloud.Network
import app.qingjian.inputmethod.cloud.SecretStore
import app.qingjian.inputmethod.core.Engine
import app.qingjian.inputmethod.settings.Settings
import app.qingjian.inputmethod.voice.VoiceConfig

/**
 * 联网服务的配置交给引擎：云联想、释义兜底、快捷翻译用的大模型接口，与语音识别；配置页改了跟着换。
 * 剪贴板的局域网同步不在这里（`ClipboardSync`）。
 */
class CloudServices(private val context: Context, private val engine: Engine) {
    /** 云端配置（地址、模型、密钥）在设置页改了，云联想、释义兜底跟着换。 */
    private val cloudListener = SharedPreferences.OnSharedPreferenceChangeListener { _, _ -> sync() }

    /** 「语音识别」设置页改了，识别方跟着换。 */
    private val voiceListener = SharedPreferences.OnSharedPreferenceChangeListener { _, _ -> syncVoice() }

    fun start() {
        SecretStore.prefs(context).registerOnSharedPreferenceChangeListener(cloudListener)
        VoiceConfig.prefs(context).registerOnSharedPreferenceChangeListener(voiceListener)
    }

    fun stop() {
        SecretStore.prefs(context).unregisterOnSharedPreferenceChangeListener(cloudListener)
        VoiceConfig.prefs(context).unregisterOnSharedPreferenceChangeListener(voiceListener)
    }

    /** 「语音识别」配好了就交给引擎；解密钥走 Keystore，配置没变 [Engine] 不重交。 */
    fun syncVoice() {
        engine.setVoice(VoiceConfig.load(context))
    }

    /**
     * 按设置接云端：释义兜底开着就交配置；云联想开着且网络允许（「只在 Wi-Fi 下使用」）就连联想一起开。
     * 两样都不要时不去接，等快捷翻译用到再接（[translate]）；之前开着联想、现在不该开了就退回只翻译。
     * 配置没变 [Engine] 不重交。返回能不能翻译。
     */
    fun sync(settings: Settings = Settings.load(context), translate: Boolean = false): Boolean {
        // 解密钥要走 Keystore，用到才读
        val cloud by lazy { CloudConfig.load(context) }
        engine.setGlossFill(if (settings.glossFill) cloud else null)
        val predict = settings.cloudPredict && (!settings.cloudWifiOnly || Network.onWifi(context))
        if (!predict && !translate && !engine.predicting) return engine.cloudConnected
        return engine.setCloud(cloud, predict)
    }
}
