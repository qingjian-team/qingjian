package app.qingjian.inputmethod

import android.app.Application
import app.qingjian.inputmethod.core.AppLog

/** 应用进程的入口：输入法、设置页、翻译都在这一个进程里，日志在这里先装上。 */
class QingjianApp : Application() {
    override fun onCreate() {
        super.onCreate()
        AppLog.init(this)
    }
}
