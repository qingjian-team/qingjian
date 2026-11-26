//! 工具栏中间的验证码与系统自动填充胶囊，横向滚动且不占用两端入口。

package app.qingjian.inputmethod.service

import android.content.Context
import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.Rect
import android.graphics.drawable.GradientDrawable
import android.graphics.drawable.RippleDrawable
import android.os.Bundle
import android.os.LocaleList
import android.text.TextUtils
import android.util.Size
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.view.inputmethod.InlineSuggestionsResponse
import android.view.inputmethod.InlineSuggestionsRequest
import android.widget.FrameLayout
import android.widget.HorizontalScrollView
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.inline.InlinePresentationSpec
import androidx.autofill.inline.UiVersions
import androidx.autofill.inline.common.ImageViewStyle
import androidx.autofill.inline.common.TextViewStyle
import androidx.autofill.inline.common.ViewStyle
import androidx.autofill.inline.v1.InlineSuggestionUi
import app.qingjian.inputmethod.theme.Palette

/** 自动填充内容由系统托管；这里只控制胶囊排列与触摸，不读取账号或密码。 */
class InlineSuggestionsHost(context: Context, private val palette: Palette) : HorizontalScrollView(context) {
    var onChanged: (() -> Unit)? = null
    private var responseGeneration = 0
    private val density = resources.displayMetrics.density
    private fun dp(value: Int) = (value * density).toInt()
    private val row = LinearLayout(context).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
    }
    private var otp: View? = null
    private val viewport = Rect()
    private val location = IntArray(2)

    val hasSuggestions: Boolean get() = (0 until row.childCount).any { row.getChildAt(it).visibility == VISIBLE }

    init {
        isHorizontalScrollBarEnabled = false
        isFillViewport = false
        overScrollMode = OVER_SCROLL_NEVER
        isFocusable = false
        setPadding(dp(8), 0, dp(8), 0)
        clipToPadding = true
        addView(row, LayoutParams(LayoutParams.WRAP_CONTENT, LayoutParams.MATCH_PARENT))
        visibility = GONE
    }

    fun show(response: InlineSuggestionsResponse) {
        responseGeneration++
        for (index in row.childCount - 1 downTo 0) {
            if (row.getChildAt(index) !== otp) row.removeViewAt(index)
        }
        onChanged?.invoke()
        val generation = responseGeneration
        // 先放占位，加载回调即使乱序返回也按服务原本的顺序排列。
        response.inlineSuggestions.forEach { suggestion ->
            val slot = FrameLayout(context).apply {
                background = capsule()
                clipToOutline = true
                visibility = GONE
            }
            row.addView(slot, itemParams())
            try {
                suggestion.inflate(context, Size(ViewGroup.LayoutParams.WRAP_CONTENT, dp(40)), context.mainExecutor) { content ->
                    if (content == null || generation != responseGeneration) return@inflate
                    val size = content.layoutParams
                    slot.addView(content, FrameLayout.LayoutParams(size.width, size.height, Gravity.CENTER))
                    slot.visibility = VISIBLE
                    onChanged?.invoke()
                }
            } catch (_: IllegalStateException) {
                // 一个 InlineSuggestion 只能加载一次；不可用的条目留空，其余建议仍可用。
            }
        }
    }

    fun showOtp(label: String, action: () -> Unit) {
        clearOtp()
        otp = TextView(context).apply {
            text = label
            contentDescription = label
            setTextColor(palette.text)
            textSize = 14f
            gravity = Gravity.CENTER
            setSingleLine()
            ellipsize = TextUtils.TruncateAt.END
            maxWidth = maximumWidth()
            setPadding(dp(14), 0, dp(14), 0)
            background = RippleDrawable(ColorStateList.valueOf(palette.accent), capsule(), null)
            isFocusable = false
            setOnClickListener { clearOtp(); action() }
        }.also { row.addView(it, 0, itemParams()) }
        scrollTo(0, 0)
        onChanged?.invoke()
    }

    fun clearOtp() {
        otp?.let(row::removeView)
        otp = null
        onChanged?.invoke()
    }

    fun clear() {
        responseGeneration++
        clearOtp()
        row.removeAllViews()
        scrollTo(0, 0)
        onChanged?.invoke()
    }

    override fun onDetachedFromWindow() {
        clear()
        super.onDetachedFromWindow()
    }

    override fun onLayout(changed: Boolean, left: Int, top: Int, right: Int, bottom: Int) {
        super.onLayout(changed, left, top, right, bottom)
        clipRemoteContent()
    }

    override fun onScrollChanged(left: Int, top: Int, oldLeft: Int, oldTop: Int) {
        super.onScrollChanged(left, top, oldLeft, oldTop)
        clipRemoteContent()
    }

    /** 远端 Surface 不遵循父视图的 Canvas 裁剪，必须同步它自己的局部裁剪范围。 */
    private fun clipRemoteContent() {
        getLocationOnScreen(location)
        viewport.set(location[0] + paddingLeft, location[1], location[0] + width - paddingRight, location[1] + height)
        for (index in 0 until row.childCount) {
            val slot = row.getChildAt(index) as? FrameLayout ?: continue
            val content = slot.getChildAt(0) ?: continue
            content.getLocationOnScreen(location)
            val clip = Rect(0, 0, content.width, content.height)
            if (!clip.intersect(viewport.left - location[0], viewport.top - location[1],
                    viewport.right - location[0], viewport.bottom - location[1])) clip.setEmpty()
            content.clipBounds = clip
        }
    }

    private fun capsule() = GradientDrawable().apply {
        setColor(palette.functionalKey)
        cornerRadius = dp(20).toFloat()
    }

    private fun itemParams() = LinearLayout.LayoutParams(LayoutParams.WRAP_CONTENT, dp(40)).apply {
        marginEnd = dp(6)
        gravity = Gravity.CENTER_VERTICAL
    }

    private fun maximumWidth(): Int = minOf(dp(240), (resources.displayMetrics.widthPixels - dp(146)).coerceAtLeast(dp(48)))

    companion object {
        /** 远端视图的尺寸在加载前协商，不能事后强行拉伸远端 Surface。 */
        fun request(context: Context, uiExtras: Bundle): InlineSuggestionsRequest? {
            val versions = runCatching { UiVersions.getVersions(uiExtras) }.getOrDefault(emptyList())
            if (versions.isNotEmpty() && UiVersions.INLINE_UI_VERSION_1 !in versions) return null
            val palette = Palette.of(context)
            val metrics = context.resources.displayMetrics
            fun dp(value: Int) = (value * metrics.density).toInt()
            val width = minOf(dp(240), (metrics.widthPixels - dp(146)).coerceAtLeast(dp(48)))
            val chip = ViewStyle.Builder().setBackgroundColor(Color.TRANSPARENT)
                .setPadding(dp(12), 0, dp(12), 0).setLayoutMargin(0, 0, 0, 0).build()
            val icon = ImageViewStyle.Builder().setMaxWidth(dp(20)).setMaxHeight(dp(20))
                .setLayoutMargin(0, 0, 0, 0).build()
            val style = InlineSuggestionUi.newStyleBuilder()
                .setChipStyle(chip).setSingleIconChipStyle(chip)
                .setStartIconStyle(icon).setEndIconStyle(icon).setSingleIconChipIconStyle(icon)
                .setTitleStyle(TextViewStyle.Builder().setTextColor(palette.text).setTextSize(14f)
                    .setLayoutMargin(dp(4), 0, dp(4), 0).build())
                .setSubtitleStyle(TextViewStyle.Builder().setTextColor(palette.gloss).setTextSize(12f)
                    .setLayoutMargin(dp(4), 0, dp(4), 0).build()).build()
            val spec = InlinePresentationSpec.Builder(Size(0, dp(40)), Size(width, dp(40)))
                .setStyle(UiVersions.newStylesBuilder().addStyle(style).build()).build()
            return InlineSuggestionsRequest.Builder(listOf(spec)).setMaxSuggestionCount(5)
                .setSupportedLocales(LocaleList.getDefault()).build()
        }
    }
}
