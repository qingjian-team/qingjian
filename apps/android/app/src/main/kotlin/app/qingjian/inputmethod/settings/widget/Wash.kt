package app.qingjian.inputmethod.settings.widget

import android.content.Context
import android.graphics.drawable.GradientDrawable
import android.view.View
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.widget.FrameLayout
import app.qingjian.inputmethod.theme.AppColors

/** 页面底子：灰底，顶上一截从淡青绿渐变下来，标题栏与滚动的内容都叠在上面。 */
object Wash {
    private const val HEIGHT_DP = 300

    fun behind(context: Context, colors: AppColors, content: View) = FrameLayout(context).apply {
        setBackgroundColor(colors.background)
        addView(View(context).apply {
            background = GradientDrawable(GradientDrawable.Orientation.TOP_BOTTOM, intArrayOf(colors.wash, colors.background))
        }, FrameLayout.LayoutParams(MATCH_PARENT, (HEIGHT_DP * context.resources.displayMetrics.density).toInt()))
        addView(content, FrameLayout.LayoutParams(MATCH_PARENT, MATCH_PARENT))
    }
}
