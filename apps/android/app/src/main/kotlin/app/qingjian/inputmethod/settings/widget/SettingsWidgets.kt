package app.qingjian.inputmethod.settings.widget

import android.app.AlertDialog
import android.content.Context
import android.content.res.ColorStateList
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Typeface
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.graphics.drawable.RippleDrawable
import android.text.InputType
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.view.ViewOutlineProvider
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.SeekBar
import android.widget.Switch
import android.widget.TextView
import app.qingjian.inputmethod.panel.base.PanelIcons
import app.qingjian.inputmethod.panel.base.PanelIcons.Icon
import app.qingjian.inputmethod.theme.AppColors

/**
 * App 页面的积木：白色大圆角卡片、粗黑分节标题、彩色图标块、设置入口行、开关、滑杆、单选。
 * 控件用系统原生的，颜色按 [AppColors] 染。
 */
class SettingsWidgets(private val context: Context, val colors: AppColors) {
    private val density = context.resources.displayMetrics.density

    fun dp(value: Int) = (value * density).toInt()

    fun text(value: String, sp: Float, color: Int, bold: Boolean = false) = TextView(context).apply {
        text = value
        setTextSize(TypedValue.COMPLEX_UNIT_SP, sp)
        setTextColor(color)
        if (bold) typeface = Typeface.DEFAULT_BOLD
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

    /** 卡片外的分节标题：粗黑大字。 */
    fun sectionTitle(title: Int) = text(context.getString(title), 18f, colors.text, bold = true).apply {
        setPadding(dp(4), dp(26), dp(4), dp(12))
    }

    fun divider() = View(context).apply {
        setBackgroundColor(colors.divider)
        layoutParams = LinearLayout.LayoutParams(MATCH_PARENT, maxOf(1, dp(1) / 2)).apply {
            marginStart = dp(18)
            marginEnd = dp(18)
        }
    }

    /** 线条图标（与键盘工具栏同一套）。 */
    fun icon(icon: Icon, color: Int, sizeDp: Float) = object : View(context) {
        private val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply { this.color = color }

        override fun onDraw(canvas: Canvas) = PanelIcons.draw(canvas, icon, width / 2f, height / 2f, sizeDp * density, paint)
    }.apply { layoutParams = LinearLayout.LayoutParams(dp(sizeDp.toInt()), dp(sizeDp.toInt())) }

    /** 彩色图标块：上浅下深的圆角色块里一个白色线条图标；[tone] 是 [AppColors.tiles] 的下标。 */
    fun tile(icon: Icon, tone: Int, sizeDp: Int) = FrameLayout(context).apply {
        val base = colors.tiles[tone % colors.tiles.size]
        background = GradientDrawable(GradientDrawable.Orientation.TOP_BOTTOM, intArrayOf(lighten(base), base)).apply {
            cornerRadius = sizeDp * 0.32f * density
        }
        addView(icon(icon, 0xFFFFFFFF.toInt(), sizeDp * 0.56f), FrameLayout.LayoutParams(dp((sizeDp * 0.56f).toInt()), dp((sizeDp * 0.56f).toInt()), Gravity.CENTER))
        layoutParams = LinearLayout.LayoutParams(dp(sizeDp), dp(sizeDp))
    }

    /** 宫格里的一项：彩色图标块，下面一行字。 */
    fun gridItem(icon: Icon, tone: Int, label: Int, onClick: () -> Unit) = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        gravity = Gravity.CENTER_HORIZONTAL
        setPadding(0, dp(16), 0, dp(14))
        background = pressable(CARD_RADIUS)
        setOnClickListener { onClick() }
        addView(tile(icon, tone, 46))
        addView(text(context.getString(label), 13f, colors.text).apply {
            gravity = Gravity.CENTER
            setPadding(0, dp(10), 0, 0)
        }, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))
    }

    /** 设置入口行：小图标块、粗体标题、一行灰字摘要、右箭头。 */
    fun entry(icon: Icon, tone: Int, title: Int, summary: String?, onClick: () -> Unit) = LinearLayout(context).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        setPadding(dp(18), dp(16), dp(14), dp(16))
        background = pressable()
        setOnClickListener { onClick() }
        addView(tile(icon, tone, 34))
        addView(LinearLayout(context).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(14), 0, dp(8), 0)
            addView(text(context.getString(title), 16f, colors.text, bold = true))
            summary?.let { addView(text(it, 12.5f, colors.secondary).apply { setPadding(0, dp(5), 0, 0) }) }
        }, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
        addView(icon(Icon.CHEVRON, colors.faint, 20f))
    }

    /** tab 页固定在顶上的大标题（生词本、我的）。 */
    fun pageTitle(title: Int) = text(context.getString(title), 20f, colors.text, bold = true).apply {
        setPadding(dp(4), dp(10), 0, dp(10))
    }

    /**
     * 顶栏随滚动浮起来：内容在顶上时透明、露出渐变；往下滚一点就渐渐铺上白底与阴影，和滚到它底下的内容分出层次。
     * 顶栏要排在 [scroll] 前面、同一个父容器里，阴影才压得住内容。
     */
    fun liftOnScroll(bar: View, scroll: ScrollView) {
        val fill = ColorDrawable(colors.surface)
        bar.background = fill
        bar.outlineProvider = ViewOutlineProvider.BOUNDS
        fun lift(y: Int) {
            val fraction = (y / (LIFT_DISTANCE_DP * density)).coerceIn(0f, 1f)
            fill.alpha = (fraction * 255).toInt()
            bar.elevation = fraction * BAR_ELEVATION_DP * density
        }
        lift(scroll.scrollY)
        scroll.setOnScrollChangeListener { _, _, y, _, _ -> lift(y) }
    }

    /** 二级页的顶栏：左边返回，标题居中。 */
    fun topBar(title: String, onBack: () -> Unit) = FrameLayout(context).apply {
        setPadding(0, dp(4), 0, dp(8))
        addView(icon(Icon.BACK, colors.text, 26f).apply {
            setOnClickListener { onBack() }
        }, FrameLayout.LayoutParams(dp(44), dp(44), Gravity.START or Gravity.CENTER_VERTICAL))
        addView(text(title, 18f, colors.text, bold = true), FrameLayout.LayoutParams(WRAP_CONTENT, WRAP_CONTENT, Gravity.CENTER))
    }

    fun row(title: Int, note: Int?) = row(context.getString(title), note?.let(context::getString))

    fun row(title: String, note: String?) = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(dp(18), dp(16), dp(18), dp(if (note == null) 6 else 16))
        addView(text(title, 16f, colors.text))
        note?.let { addView(text(it, 12.5f, colors.secondary).apply { setPadding(0, dp(6), 0, 0) }) }
    }

    fun note(text: Int) = text(context.getString(text), 12.5f, colors.secondary).apply {
        setPadding(dp(18), dp(4), dp(18), dp(16))
        setLineSpacing(0f, 1.2f)
    }

    fun choice(title: Int, selected: Boolean, onClick: () -> Unit) = LinearLayout(context).apply {
        setOnClickListener { onClick() }
        background = pressable()
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        setPadding(dp(18), dp(14), dp(18), dp(14))
        addView(text(context.getString(title), 16f, colors.text), LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
        addView(View(context).apply {
            background = GradientDrawable().apply {
                shape = GradientDrawable.OVAL
                if (selected) {
                    setColor(this@SettingsWidgets.colors.brand)
                    setStroke(dp(5), this@SettingsWidgets.colors.brandSoft)
                } else {
                    setStroke(dp(2), this@SettingsWidgets.colors.faint)
                }
            }
        }, LinearLayout.LayoutParams(dp(22), dp(22)))
    }

    fun toggle(title: Int, note: Int?, checked: Boolean, onChange: (Boolean) -> Unit) =
        toggle(context.getString(title), note?.let(context::getString), checked, null, onChange)

    /** 带开关的一行；[trailing] 放在开关左边（词库的「删除」）。 */
    fun toggle(title: String, note: String?, checked: Boolean, trailing: View?, onChange: (Boolean) -> Unit) =
        LinearLayout(context).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            background = pressable()
            val switch = Switch(context).apply {
                isChecked = checked
                thumbTintList = ColorStateList(
                    arrayOf(intArrayOf(android.R.attr.state_checked), intArrayOf()),
                    intArrayOf(colors.brand, if (colors.dark) colors.secondary else colors.surface),
                )
                trackTintList = ColorStateList(
                    arrayOf(intArrayOf(android.R.attr.state_checked), intArrayOf()),
                    intArrayOf(colors.brand, colors.faint),
                )
                setOnCheckedChangeListener { _, value -> onChange(value) }
            }
            addView(row(title, note).apply {
                setPadding(dp(18), dp(16), dp(8), dp(16))
            }, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
            trailing?.let(::addView)
            addView(switch, LinearLayout.LayoutParams(WRAP_CONTENT, WRAP_CONTENT).apply {
                marginEnd = dp(16)
            })
            setOnClickListener { switch.toggle() }
        }

    /** 点开弹单选框的一行：左边标题，右边当前选的与一个 ›。 */
    fun picker(title: Int, value: String, options: List<String>, selected: Int, onPick: (Int) -> Unit) = LinearLayout(context).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        background = pressable()
        setPadding(dp(18), dp(14), dp(12), dp(14))
        addView(text(context.getString(title), 16f, colors.text), LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
        addView(text(value, 14f, colors.secondary))
        addView(icon(Icon.CHEVRON, colors.faint, 20f), LinearLayout.LayoutParams(dp(24), dp(24)))
        setOnClickListener {
            AlertDialog.Builder(context)
                .setTitle(title)
                .setSingleChoiceItems(options.toTypedArray(), selected) { dialog, which ->
                    dialog.dismiss()
                    onPick(which)
                }
                .show()
        }
    }

    /** 点开弹多选框的一行：右边是 [value]（选了几项这类概括），点「确定」交出勾选结果。 */
    fun multiPicker(title: Int, value: String, options: List<String>, checked: BooleanArray, onDone: (BooleanArray) -> Unit) =
        LinearLayout(context).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            background = pressable()
            setPadding(dp(18), dp(14), dp(12), dp(14))
            addView(text(context.getString(title), 16f, colors.text), LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
            addView(text(value, 14f, colors.secondary))
            addView(icon(Icon.CHEVRON, colors.faint, 20f), LinearLayout.LayoutParams(dp(24), dp(24)))
            setOnClickListener {
                val picked = checked.copyOf()
                AlertDialog.Builder(context)
                    .setTitle(title)
                    .setMultiChoiceItems(options.toTypedArray(), picked) { _, which, on -> picked[which] = on }
                    .setPositiveButton(android.R.string.ok) { _, _ -> onDone(picked) }
                    .setNegativeButton(android.R.string.cancel, null)
                    .show()
            }
        }

    /**
     * 点开弹输入框的一行：左边标题，右边 [value]（密钥这类只给提示）。输入框里预填 [current]；
     * [secret] 时按密码显示、不预填。点「保存」交出去（去掉首尾空白）。
     */
    fun input(title: Int, value: String, current: String, secret: Boolean, onSave: (String) -> Unit) = LinearLayout(context).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        background = pressable()
        setPadding(dp(18), dp(14), dp(12), dp(14))
        addView(text(context.getString(title), 16f, colors.text), LinearLayout.LayoutParams(WRAP_CONTENT, WRAP_CONTENT))
        addView(text(value, 14f, colors.secondary).apply {
            gravity = Gravity.END
            maxLines = 1
            ellipsize = android.text.TextUtils.TruncateAt.MIDDLE
            setPadding(dp(12), 0, 0, 0)
        }, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
        addView(icon(Icon.CHEVRON, colors.faint, 20f), LinearLayout.LayoutParams(dp(24), dp(24)))
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

    /** 点了做一件事的一行（测试连接、清除密钥）：左边标题，右边 [value]（结果或说明，可空）。 */
    fun action(title: Int, value: String?, color: Int = colors.text, onClick: () -> Unit) = LinearLayout(context).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        background = pressable()
        setPadding(dp(18), dp(14), dp(18), dp(14))
        addView(text(context.getString(title), 16f, color), LinearLayout.LayoutParams(WRAP_CONTENT, WRAP_CONTENT))
        addView(text(value.orEmpty(), 13.5f, colors.secondary).apply {
            gravity = Gravity.END
            setPadding(dp(12), 0, 0, 0)
        }, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
        setOnClickListener { onClick() }
    }

    fun slider(
        title: Int, note: Int, min: Int, max: Int, value: Int, step: Int,
        format: (Int) -> String, onChange: (Int) -> Unit,
    ) = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(0, 0, 0, dp(16))
        val label = text(format(value), 16f, colors.brand, bold = true)
        addView(LinearLayout(context).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(18), dp(16), dp(18), dp(4))
            addView(text(context.getString(title), 16f, colors.text), LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
            addView(label)
        })
        addView(SeekBar(context).apply {
            this.max = (max - min) / step
            progress = (value - min) / step
            progressTintList = ColorStateList.valueOf(colors.brand)
            thumbTintList = ColorStateList.valueOf(colors.brand)
            progressBackgroundTintList = ColorStateList.valueOf(colors.faint)
            setPadding(dp(26), dp(12), dp(26), dp(12))
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
        addView(text(context.getString(note), 12.5f, colors.secondary).apply {
            setPadding(dp(18), 0, dp(18), 0)
            setLineSpacing(0f, 1.2f)
        })
    }

    /** 图标块的上半截：往白里调一点，做出上浅下深。 */
    private fun lighten(color: Int): Int {
        fun channel(shift: Int) = (color shr shift and 0xFF).let { it + (255 - it) * 22 / 100 }
        return (0xFF shl 24) or (channel(16) shl 16) or (channel(8) shl 8) or channel(0)
    }

    companion object {
        const val CARD_RADIUS = 20f
        private const val LIFT_DISTANCE_DP = 24f
        private const val BAR_ELEVATION_DP = 3f
    }
}
