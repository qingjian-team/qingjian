package app.qingjian.inputmethod.settings.page

import android.content.res.Configuration
import android.graphics.drawable.GradientDrawable
import android.hardware.input.InputManager
import android.view.InputDevice
import android.view.Gravity
import android.view.View
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.FrameLayout
import android.widget.LinearLayout
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.core.store.ThemeChoice
import app.qingjian.inputmethod.core.store.Themes
import app.qingjian.inputmethod.settings.Settings
import app.qingjian.inputmethod.settings.widget.ThemePreview
import app.qingjian.inputmethod.theme.Appearance
import app.qingjian.inputmethod.theme.Palette
import app.qingjian.inputmethod.theme.TextSize

/**
 * 外观二级页（对应 PC 设置程序的「外观」页）：当前主题的预览、深浅色、主题卡片、候选字号。
 * 主题与键盘配色都来自 PC 的主题库，键盘、候选条、面板与浮动编辑条一起换。改了马上写设置，开着的键盘跟着重建。
 */
class AppearanceActivity : PageActivity(), InputManager.InputDeviceListener {
    override val title = R.string.page_appearance

    private val inputManager by lazy { getSystemService(InputManager::class.java) }
    private var physicalPreview = false

    override fun onStart() {
        super.onStart()
        inputManager?.registerInputDeviceListener(this, null)
        refreshKeyboard()
    }

    override fun onStop() {
        inputManager?.unregisterInputDeviceListener(this)
        super.onStop()
    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        refreshKeyboard()
    }

    override fun onInputDeviceAdded(deviceId: Int) = refreshKeyboard()
    override fun onInputDeviceChanged(deviceId: Int) = refreshKeyboard()
    override fun onInputDeviceRemoved(deviceId: Int) = refreshKeyboard()

    private fun hasPhysicalKeyboard(): Boolean =
        inputManager?.inputDeviceIds?.any { id ->
            inputManager?.getInputDevice(id)?.let { !it.isVirtual && it.keyboardType == InputDevice.KEYBOARD_TYPE_ALPHABETIC && it.supportsSource(InputDevice.SOURCE_KEYBOARD) } == true
        } == true || resources.configuration.let {
            it.keyboard == Configuration.KEYBOARD_QWERTY && it.hardKeyboardHidden != Configuration.HARDKEYBOARDHIDDEN_YES
        }

    private fun refreshKeyboard() {
        if (physicalPreview != hasPhysicalKeyboard()) rebuild()
    }

    override fun content(): List<View> {
        physicalPreview = hasPhysicalKeyboard()
        val settings = Settings.load(this)
        val dark = settings.appearance.dark(this)
        return listOf(
            spaced(preview(settings, dark), 20),
            ui.group(
                null,
                ui.segmentedRow(R.string.section_appearance, Appearance.entries.map { getString(it.label) }, settings.appearance.ordinal) {
                    prefs.edit().putString(Settings.KEY_APPEARANCE, Appearance.entries[it].key).apply()
                    Appearance.entries[it].applyToApp(this)
                    rebuild()
                },
                footer = R.string.appearance_note,
            ),
            ui.sectionTitle(R.string.section_theme),
            themeGrid(settings, dark),
            ui.group(
                null,
                ui.segmentedRow(R.string.section_text_size, TextSize.entries.map { getString(it.label) }, settings.textSize.ordinal) {
                    prefs.edit().putString(Settings.KEY_TEXT_SIZE, TextSize.entries[it].key).apply()
                    rebuild()
                },
                footer = R.string.text_size_note,
            ),
        )
    }

    /** 当前主题、外观与字号；连接物理键盘时直接预览桌面候选窗。 */
    private fun preview(settings: Settings, dark: Boolean): View {
        val density = resources.displayMetrics.density
        val strip = Themes.preview(settings.theme, dark, density, settings.textSize.scale, physicalPreview)
        return framed(ThemePreview(this, strip, Palette.of(settings.theme, dark), aspect = 0.42f, keyRadiusDp = 6f,
            physical = physicalPreview), selected = false)
    }

    /** 主题卡片两列排：缩小的预览加名字，选中的描品牌色边。 */
    private fun themeGrid(settings: Settings, dark: Boolean): View {
        val column = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(ui.dp(14), ui.dp(14), ui.dp(14), ui.dp(4))
        }
        Themes.choices.chunked(2).forEach { pair ->
            val row = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL }
            pair.forEachIndexed { i, theme ->
                row.addView(themeCard(theme, theme.id == settings.theme, dark), LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f).apply {
                    if (i == 1) marginStart = ui.dp(12)
                })
            }
            if (pair.size == 1) row.addView(View(this), LinearLayout.LayoutParams(0, 0, 1f).apply { marginStart = ui.dp(12) })
            column.addView(row)
        }
        return ui.card(column)
    }

    private fun themeCard(theme: ThemeChoice, selected: Boolean, dark: Boolean): View {
        val strip = Themes.preview(theme.id, dark, resources.displayMetrics.density * 0.62f, 1f, physicalPreview)
        val preview = ThemePreview(this, strip, Palette.of(theme.id, dark), aspect = 0.4f, keyRadiusDp = 3f,
            physical = physicalPreview)
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER_HORIZONTAL
            setPadding(0, 0, 0, ui.dp(14))
            setOnClickListener {
                prefs.edit().putString(Settings.KEY_THEME, theme.id).apply()
                rebuild()
            }
            addView(framed(preview, selected), LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))
            addView(ui.text(theme.name, 14f, if (selected) ui.colors.brand else ui.colors.text, bold = selected).apply {
                setPadding(0, ui.dp(8), 0, 0)
            })
        }
    }

    /** 圆角裁好的预览；选中时外面一圈品牌色。 */
    private fun framed(preview: View, selected: Boolean): View {
        val radius = 10f * resources.displayMetrics.density
        val border = if (selected) ui.dp(2) else maxOf(1, ui.dp(1) / 2)
        return FrameLayout(this).apply {
            background = GradientDrawable().apply {
                cornerRadius = radius
                setColor(ui.colors.surface)
                setStroke(border, if (selected) ui.colors.brand else ui.colors.divider)
            }
            setPadding(border, border, border, border)
            clipToOutline = true
            addView(preview, FrameLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))
        }
    }
}
