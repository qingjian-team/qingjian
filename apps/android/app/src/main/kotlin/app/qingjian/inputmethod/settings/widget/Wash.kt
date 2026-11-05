package app.qingjian.inputmethod.settings.widget

import android.content.Context
import android.view.View
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.widget.FrameLayout
import app.qingjian.inputmethod.theme.AppColors

/** 页面底子：一层纯色底，标题栏与滚动的内容叠在上面。 */
object Wash {
    fun behind(context: Context, colors: AppColors, content: View) = FrameLayout(context).apply {
        setBackgroundColor(colors.background)
        addView(content, FrameLayout.LayoutParams(MATCH_PARENT, MATCH_PARENT))
    }
}
