package app.qingjian.inputmethod.settings.widget

import android.app.AlertDialog
import android.content.Context
import android.content.res.ColorStateList
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Typeface
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.graphics.drawable.LayerDrawable
import android.graphics.drawable.RippleDrawable
import android.text.InputType
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.SeekBar
import android.widget.TextView
import app.qingjian.inputmethod.panel.base.PanelIcons
import app.qingjian.inputmethod.panel.base.PanelIcons.Icon
import app.qingjian.inputmethod.theme.AppColors

/**
 * App 页面的积木（「素笺」）：分组卡片（上面一行灰色小标题、下面一行灰色脚注）、设置入口行、开关、分段选择、滑杆、单选。
 * 页面一般由若干 [group] 排成一列；颜色按 [AppColors] 染。
 */
class SettingsWidgets(private val context: Context, val colors: AppColors) {
    private val density = context.resources.displayMetrics.density

    fun dp(value: Int) = (value * density).toInt()

    fun text(value: String, sp: Float, color: Int, bold: Boolean = false) = TextView(context).apply {
        text = value
        setTextSize(TypedValue.COMPLEX_UNIT_SP, sp)
        setTextColor(color)
        if (bold) typeface = Typeface.create(Typeface.DEFAULT, Typeface.BOLD)
        includeFontPadding = false
    }

    fun rounded(color: Int, radiusDp: Float) = GradientDrawable().apply {
        setColor(color)
        cornerRadius = radiusDp * density
    }

    /** 按下时的水波纹，按 [radiusDp] 的圆角裁。 */
    fun pressable(radiusDp: Float = 0f) = RippleDrawable(
        ColorStateList.valueOf(colors.divider),
        null,
        if (radiusDp > 0f) rounded(colors.surface, radiusDp) else ColorDrawable(colors.surface),
    )

    fun card(vararg children: View) = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        background = rounded(colors.surface, CARD_RADIUS)
        clipToOutline = true
        children.forEach(::addView)
    }

    /**
     * 一组设置：灰色小标题、白色卡片（行与行之间自动加细线）、灰色脚注，标题与脚注都可省。
     * 带图标块的入口行（[entry]）之间的细线从文字那里起，其余从卡片边起。
     */
    fun group(title: String?, rows: List<View>, footer: String? = null) = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(0, dp(GROUP_GAP_DP), 0, 0)
        title?.let { addView(label(it)) }
        val card = card()
        rows.forEachIndexed { index, row ->
            if (index > 0) card.addView(divider(if (row.tag == ICON_ROW) ICON_ROW_INSET_DP else 16))
            card.addView(row)
        }
        addView(card)
        footer?.let { addView(footnote(it)) }
    }

    fun group(title: Int?, vararg rows: View, footer: Int? = null) =
        group(title?.let(context::getString), rows.toList(), footer?.let(context::getString))

    /** 卡片上方的灰色小标题。 */
    fun label(value: String) = text(value, 13f, colors.secondary).apply {
        setPadding(dp(16), 0, dp(16), dp(7))
    }

    /** 不带卡片的小标题（外观页主题列表上方）。 */
    fun sectionTitle(title: Int) = label(context.getString(title)).apply {
        setPadding(dp(16), dp(GROUP_GAP_DP), dp(16), dp(7))
    }

    /** 卡片下方的灰色脚注。 */
    fun footnote(value: String) = text(value, 13f, colors.secondary).apply {
        setPadding(dp(16), dp(7), dp(16), 0)
        setLineSpacing(0f, 1.25f)
    }

    fun divider(insetDp: Int = 16) = View(context).apply {
        setBackgroundColor(colors.divider)
        layoutParams = LinearLayout.LayoutParams(MATCH_PARENT, maxOf(1, dp(1) / 2)).apply {
            marginStart = dp(insetDp)
        }
    }

    /** 线条图标（与键盘工具栏同一套）。 */
    fun icon(icon: Icon, color: Int, sizeDp: Float) = object : View(context) {
        private val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply { this.color = color }

        override fun onDraw(canvas: Canvas) = PanelIcons.draw(canvas, icon, width / 2f, height / 2f, sizeDp * density, paint)
    }.apply { layoutParams = LinearLayout.LayoutParams(dp(sizeDp.toInt()), dp(sizeDp.toInt())) }

    /** 圆角色块里一个白色线条图标；[muted] 用灰色块（次要入口）。 */
    fun tile(icon: Icon, sizeDp: Int, muted: Boolean = false) = FrameLayout(context).apply {
        background = rounded(if (muted) colors.tileMuted else colors.tile, sizeDp * 0.27f)
        val inner = (sizeDp * 0.6f).toInt()
        addView(icon(icon, 0xFFFFFFFF.toInt(), inner.toFloat()), FrameLayout.LayoutParams(dp(inner), dp(inner), Gravity.CENTER))
        layoutParams = LinearLayout.LayoutParams(dp(sizeDp), dp(sizeDp))
    }

    /** 设置入口行：小图标块、标题、右边灰字（当前值，可空）、›。 */
    fun entry(icon: Icon, title: String, value: String?, muted: Boolean = false, onClick: () -> Unit) = LinearLayout(context).apply {
        tag = ICON_ROW
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        minimumHeight = dp(ROW_HEIGHT_DP)
        setPadding(dp(16), dp(8), dp(12), dp(8))
        background = pressable()
        setOnClickListener { onClick() }
        addView(tile(icon, 30, muted))
        addView(text(title, 16f, colors.text).apply { setPadding(dp(12), 0, dp(8), 0) }, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
        value?.let { addView(valueText(it)) }
        addView(chevron())
    }

    fun entry(icon: Icon, title: Int, value: String?, muted: Boolean = false, onClick: () -> Unit) =
        entry(icon, context.getString(title), value, muted, onClick)

    /** 不带图标块的跳转行：标题、右边灰字、›。 */
    fun link(title: Int, value: String?, onClick: () -> Unit) = LinearLayout(context).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        minimumHeight = dp(ROW_HEIGHT_DP)
        setPadding(dp(16), dp(8), dp(12), dp(8))
        background = pressable()
        setOnClickListener { onClick() }
        addView(text(context.getString(title), 16f, colors.text), LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
        value?.let { addView(valueText(it)) }
        addView(chevron())
    }

    /** tab 页顶栏的标题：居中，与二级页同一个字号。 */
    fun pageTitle(title: Int) = FrameLayout(context).apply {
        minimumHeight = dp(BAR_HEIGHT_DP)
        addView(barTitle(context.getString(title)), FrameLayout.LayoutParams(WRAP_CONTENT, WRAP_CONTENT, Gravity.CENTER))
    }

    /** 顶栏与 tab 栏的底：栏底色加一道细线，[bottomLine] 时线在下边（顶栏），否则在上边（tab 栏），把中间的内容区框出来。 */
    fun barBackground(bottomLine: Boolean): Drawable {
        val line = maxOf(1, dp(1) / 2)
        return LayerDrawable(arrayOf(ColorDrawable(colors.bar), ColorDrawable(colors.divider))).apply {
            setLayerHeight(1, line)
            setLayerGravity(1, if (bottomLine) Gravity.BOTTOM else Gravity.TOP)
        }
    }

    /** 二级页的顶栏：左边返回，标题居中。 */
    fun topBar(title: String, onBack: () -> Unit) = FrameLayout(context).apply {
        minimumHeight = dp(BAR_HEIGHT_DP)
        addView(icon(Icon.BACK, colors.brand, 24f).apply {
            contentDescription = context.getString(android.R.string.cancel)
            background = RippleDrawable(ColorStateList.valueOf(colors.divider), null, null)
            setOnClickListener { onBack() }
        }, FrameLayout.LayoutParams(dp(48), dp(48), Gravity.START or Gravity.CENTER_VERTICAL))
        addView(barTitle(title), FrameLayout.LayoutParams(WRAP_CONTENT, WRAP_CONTENT, Gravity.CENTER))
    }

    private fun barTitle(title: String) = text(title, 17f, colors.text, bold = true)

    fun row(title: Int, note: Int?) = row(context.getString(title), note?.let(context::getString))

    fun row(title: String, note: String?) = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        gravity = Gravity.CENTER_VERTICAL
        minimumHeight = dp(ROW_HEIGHT_DP)
        setPadding(dp(16), dp(12), dp(16), dp(12))
        addView(text(title, 16f, colors.text))
        note?.let { addView(text(it, 13f, colors.secondary).apply { setPadding(0, dp(4), 0, 0); setLineSpacing(0f, 1.2f) }) }
    }

    fun toggle(title: Int, note: Int?, checked: Boolean, onChange: (Boolean) -> Unit) =
        toggle(context.getString(title), note?.let(context::getString), checked, null, onChange)

    /** 带开关的一行；[trailing] 放在开关左边（词库的「删除」）。 */
    fun toggle(title: String, note: String?, checked: Boolean, trailing: View?, onChange: (Boolean) -> Unit) =
        LinearLayout(context).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            background = pressable()
            val switch = switch(checked, onChange)
            addView(row(title, note).apply { setPadding(dp(16), dp(12), dp(8), dp(12)) }, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
            trailing?.let(::addView)
            addView(switch, LinearLayout.LayoutParams(WRAP_CONTENT, WRAP_CONTENT).apply { marginEnd = dp(16) })
            setOnClickListener { switch.toggle() }
        }

    private fun switch(checked: Boolean, onChange: (Boolean) -> Unit) = PillSwitch(context, colors.brand, colors.faint).apply {
        isChecked = checked
        this.onChange = onChange
    }

    /** 分段选择（几个选项并排、选中的浮起一块白）。 */
    fun segmented(options: List<String>, selected: Int, onPick: (Int) -> Unit) = LinearLayout(context).apply {
        orientation = LinearLayout.HORIZONTAL
        background = rounded(colors.fill, 9f)
        setPadding(dp(2), dp(2), dp(2), dp(2))
        options.forEachIndexed { index, option ->
            val on = index == selected
            addView(text(option, 13f, colors.text, bold = on).apply {
                gravity = Gravity.CENTER
                if (on) {
                    background = rounded(if (colors.dark) 0xFF636366.toInt() else colors.surface, 7f)
                    elevation = dp(1).toFloat()
                }
                isSelected = on
                setOnClickListener { if (!on) onPick(index) }
            }, LinearLayout.LayoutParams(0, dp(28), 1f))
        }
    }

    /** 左边标题、右边分段选择的一行；选项多或字长时分段选择单独占满一行。 */
    fun segmentedRow(title: Int, options: List<String>, selected: Int, onPick: (Int) -> Unit): View {
        val wide = options.size > 4 || options.any { it.length > 3 }
        return if (wide) {
            LinearLayout(context).apply {
                orientation = LinearLayout.VERTICAL
                setPadding(dp(16), dp(12), dp(16), dp(12))
                addView(text(context.getString(title), 16f, colors.text))
                addView(segmented(options, selected, onPick), LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT).apply { topMargin = dp(10) })
            }
        } else {
            LinearLayout(context).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                minimumHeight = dp(ROW_HEIGHT_DP)
                setPadding(dp(16), dp(8), dp(16), dp(8))
                addView(text(context.getString(title), 16f, colors.text), LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
                addView(segmented(options, selected, onPick), LinearLayout.LayoutParams(dp(SEGMENT_WIDTH_DP * options.size), WRAP_CONTENT))
            }
        }
    }

    /** 点开弹单选框的一行：左边标题，右边当前选的与一个 ›。 */
    fun picker(title: Int, value: String, options: List<String>, selected: Int, onPick: (Int) -> Unit) =
        link(title, value) {
            AlertDialog.Builder(context)
                .setTitle(title)
                .setSingleChoiceItems(options.toTypedArray(), selected) { dialog, which ->
                    dialog.dismiss()
                    onPick(which)
                }
                .show()
        }

    /** 点开弹多选框的一行：右边是 [value]（选了几项这类概括），点「确定」交出勾选结果。 */
    fun multiPicker(title: Int, value: String, options: List<String>, checked: BooleanArray, onDone: (BooleanArray) -> Unit) =
        link(title, value) {
            val picked = checked.copyOf()
            AlertDialog.Builder(context)
                .setTitle(title)
                .setMultiChoiceItems(options.toTypedArray(), picked) { _, which, on -> picked[which] = on }
                .setPositiveButton(android.R.string.ok) { _, _ -> onDone(picked) }
                .setNegativeButton(android.R.string.cancel, null)
                .show()
        }

    /**
     * 点开弹输入框的一行：左边标题，右边 [value]（密钥这类只给提示）。输入框里预填 [current]；
     * [secret] 时按密码显示、不预填。点「保存」交出去（去掉首尾空白）。
     */
    fun input(title: Int, value: String, current: String, secret: Boolean, onSave: (String) -> Unit) = LinearLayout(context).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        minimumHeight = dp(ROW_HEIGHT_DP)
        background = pressable()
        setPadding(dp(16), dp(8), dp(12), dp(8))
        addView(text(context.getString(title), 16f, colors.text), LinearLayout.LayoutParams(WRAP_CONTENT, WRAP_CONTENT))
        addView(valueText(value).apply {
            gravity = Gravity.END
            maxLines = 1
            ellipsize = android.text.TextUtils.TruncateAt.MIDDLE
            setPadding(dp(12), 0, 0, 0)
        }, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
        addView(chevron())
        setOnClickListener {
            val field = EditText(context).apply {
                setSingleLine()
                if (secret) {
                    inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD
                } else {
                    inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_URI
                    setText(current)
                    setSelection(current.length)
                }
            }
            val box = FrameLayout(context).apply {
                setPadding(dp(20), dp(8), dp(20), 0)
                addView(field)
            }
            AlertDialog.Builder(context)
                .setTitle(title)
                .setView(box)
                .setPositiveButton(android.R.string.ok) { _, _ -> onSave(field.text.toString().trim()) }
                .setNegativeButton(android.R.string.cancel, null)
                .show()
        }
    }

    /** 点了做一件事的一行（测试连接、清除密钥）：左边标题（默认品牌色），右边 [value]（结果或说明，可空）。 */
    fun action(title: Int, value: String?, color: Int = colors.brand, onClick: () -> Unit) = LinearLayout(context).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        minimumHeight = dp(ROW_HEIGHT_DP)
        background = pressable()
        setPadding(dp(16), dp(8), dp(16), dp(8))
        addView(text(context.getString(title), 16f, color), LinearLayout.LayoutParams(WRAP_CONTENT, WRAP_CONTENT))
        addView(valueText(value.orEmpty()).apply {
            gravity = Gravity.END
            setPadding(dp(12), 0, 0, 0)
        }, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
        setOnClickListener { onClick() }
    }

    fun slider(
        title: Int, note: Int?, min: Int, max: Int, value: Int, step: Int,
        format: (Int) -> String, onChange: (Int) -> Unit,
    ) = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(0, 0, 0, dp(12))
        val label = valueText(format(value))
        addView(LinearLayout(context).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(16), dp(12), dp(16), dp(2))
            addView(text(context.getString(title), 16f, colors.text), LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
            addView(label)
        })
        addView(SeekBar(context).apply {
            this.max = (max - min) / step
            progress = (value - min) / step
            progressTintList = ColorStateList.valueOf(colors.brand)
            thumbTintList = ColorStateList.valueOf(colors.brand)
            progressBackgroundTintList = ColorStateList.valueOf(colors.faint)
            setPadding(dp(24), dp(10), dp(24), dp(10))
            setOnSeekBarChangeListener(object : SeekBar.OnSeekBarChangeListener {
                override fun onProgressChanged(bar: SeekBar, progress: Int, fromUser: Boolean) {
                    val current = min + progress * step
                    label.text = format(current)
                    if (fromUser) onChange(current)
                }

                override fun onStartTrackingTouch(bar: SeekBar) = Unit
                override fun onStopTrackingTouch(bar: SeekBar) = Unit
            })
        })
        note?.let {
            addView(text(context.getString(it), 13f, colors.secondary).apply {
                setPadding(dp(16), 0, dp(16), 0)
                setLineSpacing(0f, 1.2f)
            })
        }
    }

    private fun valueText(value: String) = text(value, 15f, colors.secondary)

    private fun chevron() = icon(Icon.CHEVRON, colors.faint, 18f).apply {
        layoutParams = LinearLayout.LayoutParams(dp(20), dp(20)).apply { marginStart = dp(4) }
    }

    companion object {
        const val CARD_RADIUS = 12f
        private const val GROUP_GAP_DP = 24
        private const val ROW_HEIGHT_DP = 48
        private const val BAR_HEIGHT_DP = 52
        private const val ICON_ROW_INSET_DP = 58
        private const val SEGMENT_WIDTH_DP = 52
        private const val ICON_ROW = "icon-row"
    }
}
