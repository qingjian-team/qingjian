package app.qingjian.inputmethod.cloud

import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities

/** 云联想「只在 Wi-Fi 下使用」看的当前网络（要 `ACCESS_NETWORK_STATE`）。 */
object Network {
    /**
     * 当前默认网络走 Wi-Fi 或有线。开着 VPN 时系统把底层网络的传输方式并进 VPN 的能力里，照样认得出 Wi-Fi；
     * 读不到（没网、没权限）当作不是。
     */
    fun onWifi(context: Context): Boolean {
        val manager = context.getSystemService(ConnectivityManager::class.java) ?: return false
        val capabilities = runCatching { manager.getNetworkCapabilities(manager.activeNetwork) }.getOrNull() ?: return false
        return capabilities.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) ||
            capabilities.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET)
    }
}
