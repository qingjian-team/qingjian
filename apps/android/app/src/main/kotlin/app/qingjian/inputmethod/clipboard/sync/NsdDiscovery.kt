package app.qingjian.inputmethod.clipboard.sync

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.os.Handler
import android.os.Looper
import app.qingjian.inputmethod.core.Native
import java.net.Inet4Address

/**
 * 安卓的局域网发现，走系统的 NsdManager，和桌面端的 mDNS 互通（服务类型与 TXT 键见 crates/qingjian-clipboard-sync/src/discovery/mod.rs，改了要两边一起改）。
 * 不用 Rust 那份：安卓 11 起应用读不到网卡状态标志，mdns-sd 会把所有网卡当成没启动而跳过，既不发也不收。
 */
class NsdDiscovery(context: Context, private val deviceId: String) {
    private val manager = context.getSystemService(NsdManager::class.java)
    private val main = Handler(Looper.getMainLooper())

    /** 一次只能解析一个（安卓 14 以前的限制），排队。 */
    private val pending = ArrayDeque<NsdServiceInfo>()
    private var resolving = false
    private var stopped = false

    /** 系统给的服务名到设备 id：下线回调只带服务名。 */
    private val known = HashMap<String, String>()

    private val registration = object : NsdManager.RegistrationListener {
        override fun onRegistrationFailed(info: NsdServiceInfo, error: Int) = Unit
        override fun onUnregistrationFailed(info: NsdServiceInfo, error: Int) = Unit
        override fun onServiceRegistered(info: NsdServiceInfo) = Unit
        override fun onServiceUnregistered(info: NsdServiceInfo) = Unit
    }

    private val browsing = object : NsdManager.DiscoveryListener {
        override fun onStartDiscoveryFailed(type: String, error: Int) = Unit
        override fun onStopDiscoveryFailed(type: String, error: Int) = Unit
        override fun onDiscoveryStarted(type: String) = Unit
        override fun onDiscoveryStopped(type: String) = Unit

        override fun onServiceFound(info: NsdServiceInfo) {
            main.post {
                pending.addLast(info)
                resolveNext()
            }
        }

        override fun onServiceLost(info: NsdServiceInfo) {
            main.post {
                known.remove(info.serviceName)?.let { id -> runCatching { Native.clipboardSyncGone(id) } }
            }
        }
    }

    fun start(name: String, model: String, port: Int) {
        val manager = manager ?: return
        val info = NsdServiceInfo().apply {
            serviceName = deviceId.take(INSTANCE_LENGTH)
            serviceType = SERVICE_TYPE
            this.port = port
            setAttribute("id", deviceId)
            setAttribute("name", name)
            setAttribute("model", model)
            setAttribute("v", PROTOCOL_VERSION)
        }
        runCatching { manager.registerService(info, NsdManager.PROTOCOL_DNS_SD, registration) }
        runCatching { manager.discoverServices(SERVICE_TYPE, NsdManager.PROTOCOL_DNS_SD, browsing) }
    }

    fun stop() {
        stopped = true
        pending.clear()
        runCatching { manager?.unregisterService(registration) }
        runCatching { manager?.stopServiceDiscovery(browsing) }
    }

    @Suppress("DEPRECATION")
    private fun resolveNext() {
        if (resolving || stopped) return
        val info = pending.removeFirstOrNull() ?: return
        resolving = true
        val done = runCatching {
            manager?.resolveService(info, object : NsdManager.ResolveListener {
                override fun onResolveFailed(info: NsdServiceInfo, error: Int) = finished()

                override fun onServiceResolved(info: NsdServiceInfo) {
                    report(info)
                    finished()
                }
            })
        }
        if (done.isFailure) finished()
    }

    private fun finished() {
        main.post {
            resolving = false
            resolveNext()
        }
    }

    @Suppress("DEPRECATION")
    private fun report(info: NsdServiceInfo) {
        fun attribute(key: String) = info.attributes[key]?.toString(Charsets.UTF_8).orEmpty()
        if (attribute("v") != PROTOCOL_VERSION) return
        val id = attribute("id")
        if (id.isEmpty() || id == deviceId) return
        val host = info.host as? Inet4Address ?: return
        val address = "${host.hostAddress}:${info.port}"
        main.post {
            if (stopped) return@post
            known[info.serviceName] = id
            runCatching { Native.clipboardSyncFound(id, attribute("name"), attribute("model"), address) }
        }
    }

    private companion object {
        const val SERVICE_TYPE = "_qjclip._tcp"
        const val INSTANCE_LENGTH = 32
        const val PROTOCOL_VERSION = "1"
    }
}
