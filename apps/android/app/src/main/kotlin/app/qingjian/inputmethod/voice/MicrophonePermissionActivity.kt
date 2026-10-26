package app.qingjian.inputmethod.voice

import android.Manifest
import android.app.Activity
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Bundle

/** 申请录音权限：输入法服务弹不了权限框，开这个透明窗口代为申请，结果交回 [request] 的回调后马上关掉。 */
class MicrophonePermissionActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        if (savedInstanceState == null) requestPermissions(arrayOf(Manifest.permission.RECORD_AUDIO), REQUEST)
    }

    override fun onRequestPermissionsResult(requestCode: Int, permissions: Array<out String>, grantResults: IntArray) {
        if (requestCode != REQUEST) return
        val granted = grantResults.firstOrNull() == PackageManager.PERMISSION_GRANTED
        finish()
        pending?.invoke(granted)
        pending = null
    }

    override fun onDestroy() {
        // 窗口被系统收掉、没等到结果：当没给
        if (isFinishing && pending != null && !isChangingConfigurations) {
            pending?.invoke(false)
            pending = null
        }
        super.onDestroy()
    }

    companion object {
        private const val REQUEST = 1

        /** 等着结果的回调；同一时间只有一个申请。 */
        private var pending: ((Boolean) -> Unit)? = null

        fun granted(context: Context) =
            context.checkSelfPermission(Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED

        /** 开窗口申请，[onResult] 在主线程回（给没给）。 */
        fun request(context: Context, onResult: (Boolean) -> Unit) {
            pending = onResult
            context.startActivity(
                Intent(context, MicrophonePermissionActivity::class.java)
                    .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_NO_ANIMATION),
            )
        }
    }
}
