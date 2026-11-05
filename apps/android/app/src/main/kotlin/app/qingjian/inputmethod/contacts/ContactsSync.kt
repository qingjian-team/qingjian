package app.qingjian.inputmethod.contacts

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.database.ContentObserver
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.provider.ContactsContract
import app.qingjian.inputmethod.core.AppLog
import app.qingjian.inputmethod.core.Engine
import app.qingjian.inputmethod.settings.Settings
import kotlin.concurrent.thread
import org.json.JSONArray
import org.json.JSONObject

/**
 * 通讯录接进引擎：设置里开了、也给了读通讯录的权限，才在后台线程读一遍联系人的名字、电话、邮箱，回主线程整份交给引擎；
 * 通讯录变了（系统的 [ContentObserver]）就重读。关掉或没有权限时交空列表，引擎里的名字一起清掉。
 * 名字与号码只在内存里，不落盘、不上传。
 */
class ContactsSync(private val context: Context, private val engine: Engine) {
    private val main = Handler(Looper.getMainLooper())

    /** 上次交给引擎时的（开着, 给电话邮箱）；没交过为 null。设置页改别的设置时不重读。 */
    private var applied: Pair<Boolean, Boolean>? = null

    /** 每次读都加一，读完回到主线程时不是最新的那次就丢掉。 */
    private var generation = 0

    private var observing = false

    /** 开着却读到零个联系人的时刻（`uptimeMillis`）：权限只给了「使用中」时系统在后台给空结果，见 [retryIfEmpty]。 */
    private var emptyAt: Long? = null

    private val observer = object : ContentObserver(main) {
        override fun onChange(selfChange: Boolean) {
            main.removeCallbacks(reload)
            main.postDelayed(reload, RELOAD_DELAY_MS)
        }
    }

    private val reload = Runnable { sync(force = true) }

    /** 按设置交通讯录；引擎装好、设置改了时调。开关没变且不是 [force] 时不重读。 */
    fun sync(settings: Settings = Settings.load(context), force: Boolean = false) {
        if (!engine.ready) return
        val wanted = settings.contacts && granted(context)
        val state = wanted to settings.contactDetails
        if (!force && state == applied) return
        applied = state
        val token = ++generation
        if (!wanted) {
            emptyAt = null
            stopObserving()
            engine.setContacts("[]", false)
            return
        }
        startObserving()
        thread(name = "qingjian-contacts") {
            val json = runCatching { read(context) }
                .onFailure { AppLog.warn("contacts read failed: $it") }
                .getOrNull() ?: return@thread
            main.post {
                if (token != generation) return@post
                val names = engine.setContacts(json, settings.contactDetails)
                emptyAt = if (json == "[]") SystemClock.uptimeMillis() else null
                AppLog.info("contacts synced: $names names")
            }
        }
    }

    /** 上次开着却读到零个（输入法在后台时系统给的空结果）：键盘弹出时重读，两次至少隔 [RETRY_MS]。 */
    fun retryIfEmpty() {
        val at = emptyAt ?: return
        if (SystemClock.uptimeMillis() - at < RETRY_MS) return
        emptyAt = null
        sync(force = true)
    }

    fun stop() {
        stopObserving()
        main.removeCallbacks(reload)
    }

    private fun startObserving() {
        if (observing) return
        context.contentResolver.registerContentObserver(ContactsContract.Contacts.CONTENT_URI, true, observer)
        observing = true
    }

    private fun stopObserving() {
        if (!observing) return
        context.contentResolver.unregisterContentObserver(observer)
        observing = false
    }

    companion object {
        /** 通讯录变了以后等一会儿再重读：同步联系人时会连着来一串变化。 */
        private const val RELOAD_DELAY_MS = 2000L

        /** 读到零个后最快多久再试：读一次只要几毫秒，主要是别在连着弹键盘时一直读。 */
        private const val RETRY_MS = 1_000L

        fun granted(context: Context): Boolean =
            context.checkSelfPermission(Manifest.permission.READ_CONTACTS) == PackageManager.PERMISSION_GRANTED

        /** 读出 `[{"name":…,"details":[电话…, 邮箱…]}]`：电话去掉空格、横线与括号，排在邮箱前面。 */
        private fun read(context: Context): String {
            val resolver = context.contentResolver
            val names = LinkedHashMap<Long, String>()
            resolver.query(
                ContactsContract.Contacts.CONTENT_URI,
                arrayOf(ContactsContract.Contacts._ID, ContactsContract.Contacts.DISPLAY_NAME_PRIMARY),
                null, null, null,
            )?.use { cursor ->
                while (cursor.moveToNext()) {
                    val name = cursor.getString(1)?.trim().orEmpty()
                    if (name.isNotEmpty()) names[cursor.getLong(0)] = name
                }
            }
            val phones = HashMap<Long, MutableList<String>>()
            val emails = HashMap<Long, MutableList<String>>()
            resolver.query(
                ContactsContract.Data.CONTENT_URI,
                arrayOf(ContactsContract.Data.CONTACT_ID, ContactsContract.Data.MIMETYPE, ContactsContract.Data.DATA1),
                "${ContactsContract.Data.MIMETYPE} IN (?, ?)",
                arrayOf(
                    ContactsContract.CommonDataKinds.Phone.CONTENT_ITEM_TYPE,
                    ContactsContract.CommonDataKinds.Email.CONTENT_ITEM_TYPE,
                ),
                null,
            )?.use { cursor ->
                while (cursor.moveToNext()) {
                    val value = cursor.getString(2)?.trim().orEmpty()
                    if (value.isEmpty()) continue
                    when (cursor.getString(1)) {
                        ContactsContract.CommonDataKinds.Phone.CONTENT_ITEM_TYPE ->
                            phones.getOrPut(cursor.getLong(0)) { mutableListOf() } += value.filterNot { it in " -()" }
                        else -> emails.getOrPut(cursor.getLong(0)) { mutableListOf() } += value
                    }
                }
            }
            val array = JSONArray()
            for ((id, name) in names) {
                val details = JSONArray()
                (phones[id].orEmpty() + emails[id].orEmpty()).distinct().forEach(details::put)
                array.put(JSONObject().put("name", name).put("details", details))
            }
            return array.toString()
        }
    }
}
