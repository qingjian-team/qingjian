//! 全窗口透明宿主，将电脑候选位图放在应用光标旁。

package app.qingjian.inputmethod.service.candidates

import android.content.Context
import android.graphics.Rect
import android.graphics.RectF
import android.os.Build
import android.view.ViewGroup
import android.view.WindowInsets
import app.qingjian.inputmethod.candidates.physical.PhysicalCandidatesView

class AnchoredFrame(context: Context, private val candidate: PhysicalCandidatesView) : ViewGroup(context) {
    var caret: RectF? = null
        set(value) { field = value; requestLayout() }

    private val origin = IntArray(2)

    private val margin = (6 * resources.displayMetrics.density).toInt()

    private val safe = Rect()

    private val content = Rect()

    init {
        addView(candidate, LayoutParams(LayoutParams.WRAP_CONTENT, LayoutParams.WRAP_CONTENT))
        setOnApplyWindowInsetsListener { _, insets -> requestLayout(); insets }
    }

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val w = MeasureSpec.getSize(widthMeasureSpec)
        val h = MeasureSpec.getSize(heightMeasureSpec)
        safeBounds(w, h)
        candidate.measure(
            MeasureSpec.makeMeasureSpec(safe.width().coerceAtLeast(1), MeasureSpec.AT_MOST),
            MeasureSpec.makeMeasureSpec(safe.height().coerceAtLeast(1), MeasureSpec.AT_MOST),
        )
        setMeasuredDimension(w, h)
    }

    override fun onLayout(changed: Boolean, l: Int, t: Int, r: Int, b: Int) {
        safeBounds(width, height)
        getLocationOnScreen(origin)
        candidate.contentBounds(content)
        val anchor = caret
        val (x, y) = Placement.position(
            safe.left, safe.top, safe.right, safe.bottom, content.width(), content.height(),
            anchor?.left?.minus(origin[0]), anchor?.top?.minus(origin[1]), anchor?.bottom?.minus(origin[1]), margin,
        )
        val bitmapX = x - content.left
        val bitmapY = y - content.top
        candidate.layout(bitmapX, bitmapY, bitmapX + candidate.measuredWidth, bitmapY + candidate.measuredHeight)
    }

    private fun safeBounds(w: Int, h: Int) {
        val insets = rootWindowInsets
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R && insets != null) {
            val bars = insets.getInsets(WindowInsets.Type.systemBars() or WindowInsets.Type.displayCutout())
            safe.set(bars.left + margin, bars.top + margin, w - bars.right - margin, h - bars.bottom - margin)
        } else {
            @Suppress("DEPRECATION")
            safe.set(margin, (insets?.systemWindowInsetTop ?: 0) + margin,
                w - margin, h - (insets?.systemWindowInsetBottom ?: 0) - margin)
        }
        safe.right = safe.right.coerceAtLeast(safe.left)
        safe.bottom = safe.bottom.coerceAtLeast(safe.top)
    }
}
