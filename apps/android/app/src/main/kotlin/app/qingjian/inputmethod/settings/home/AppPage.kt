package app.qingjian.inputmethod.settings.home

import android.view.View

/** 底部 tab 的一页：[header] 是固定在顶上的标题栏，[build] 出下面可滚的内容（一列，外面由 [MainActivity] 套滚动与渐变底）。 */
interface AppPage {
    fun header(): View

    fun build(): View

    /** 页面上画着的数据过期了（切了输入法、多打了字），回到前台时要重建。 */
    fun stale(): Boolean = false
}
