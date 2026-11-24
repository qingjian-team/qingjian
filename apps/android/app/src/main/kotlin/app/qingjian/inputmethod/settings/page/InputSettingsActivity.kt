package app.qingjian.inputmethod.settings.page

import android.view.Gravity
import android.view.View
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.LinearLayout
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.core.store.SettingChoices
import app.qingjian.inputmethod.core.store.Shuangpin
import app.qingjian.inputmethod.settings.Settings
import app.qingjian.inputmethod.settings.home.SettingsPage

/**
 * 输入二级页：管「打出什么字」——中文输入方案（选了双拼 / 五笔多出各自的选项）、拼音（模糊音、纠错、分次选词）、
 * 中英混打、候选后面配的 emoji / 颜文字 / 符号、标点与字形、学习输入习惯。与 PC 设置程序「输入」「中英文」「候选」页的同名项一致；改了马上写设置，开着的键盘跟着重建。
 */
class InputSettingsActivity : PageActivity() {
    override val title = R.string.page_input

    override fun content(): List<View> {
        val settings = Settings.load(this)
        val options = settings.inputOptions
        return listOf(
            ui.group(
                getString(R.string.section_layout),
                listOf(layoutPicker(settings.layout)) + schemeRows(settings),
                getString(layoutNote(settings.layout)),
            ),
            ui.group(
                R.string.section_pinyin,
                fuzzyPicker(settings),
                ui.toggle(R.string.spelling_correction_title, R.string.spelling_correction_note, options.spellingCorrection) {
                    prefs.edit().putBoolean(Settings.KEY_SPELLING_CORRECTION, it).apply()
                },
                ui.toggle(R.string.hold_selections_title, R.string.hold_selections_note, options.holdSelections) {
                    prefs.edit().putBoolean(Settings.KEY_HOLD_SELECTIONS, it).apply()
                },
                ui.toggle(R.string.follow_ups_title, R.string.follow_ups_note, options.followUps) {
                    prefs.edit().putBoolean(Settings.KEY_FOLLOW_UPS, it).apply()
                },
                footer = R.string.fuzzy_note,
            ),
            ui.group(getString(R.string.section_mixed), mixedRows(settings), getString(R.string.english_in_chinese_note)),
            ui.group(
                R.string.section_extras,
                extraRow(R.string.extras_emoji, Settings.KEY_EMOJI, options.emoji),
                extraRow(R.string.extras_kaomoji, Settings.KEY_KAOMOJI, options.kaomoji),
                extraRow(R.string.extras_symbols, Settings.KEY_SYMBOLS, options.symbols),
                footer = R.string.extras_note,
            ),
            ui.group(
                R.string.section_punctuation,
                ui.toggle(R.string.full_width_title, R.string.full_width_note, options.fullWidthPunctuation) {
                    prefs.edit().putBoolean(Settings.KEY_FULL_WIDTH, it).apply()
                },
                ui.toggle(R.string.pair_punctuation_title, R.string.pair_punctuation_note, options.pairPunctuation) {
                    prefs.edit().putBoolean(Settings.KEY_PAIR_PUNCTUATION, it).apply()
                },
                ui.toggle(
                    R.string.english_pair_punctuation_title,
                    R.string.english_pair_punctuation_note,
                    options.englishPairPunctuation,
                ) {
                    prefs.edit().putBoolean(Settings.KEY_ENGLISH_PAIR_PUNCTUATION, it).apply()
                },
                variantPicker(settings),
                footer = R.string.traditional_variant_note,
            ),
            ui.group(
                null,
                ui.toggle(R.string.learning_title, null, options.learning) {
                    prefs.edit().putBoolean(Settings.KEY_LEARNING, it).apply()
                },
                footer = R.string.learning_note,
            ),
        )
    }

    /** 五种方案排成一行方块，选中的描品牌色边。 */
    private fun layoutPicker(current: Settings.Layout) = LinearLayout(this).apply {
        orientation = LinearLayout.HORIZONTAL
        setPadding(ui.dp(8), ui.dp(14), ui.dp(8), ui.dp(12))
        LAYOUTS.forEach { (layout, glyph) ->
            val on = layout == current
            val color = if (on) ui.colors.brand else ui.colors.text
            addView(LinearLayout(this@InputSettingsActivity).apply {
                orientation = LinearLayout.VERTICAL
                gravity = Gravity.CENTER_HORIZONTAL
                isSelected = on
                contentDescription = getString(SettingsPage.layoutName(layout))
                setOnClickListener {
                    prefs.edit().putString(Settings.KEY_LAYOUT, layout.name).apply()
                    rebuild()
                }
                addView(ui.text(glyph, 15f, color, bold = true).apply {
                    gravity = Gravity.CENTER
                    background = ui.rounded(if (on) ui.colors.brandSoft else ui.colors.surface, 8f).apply {
                        setStroke(if (on) ui.dp(2) else maxOf(1, ui.dp(1) / 2) * 2, if (on) ui.colors.brand else ui.colors.divider)
                    }
                }, LinearLayout.LayoutParams(ui.dp(52), ui.dp(40)))
                addView(ui.text(getString(SettingsPage.layoutName(layout)), 12f, color).apply {
                    gravity = Gravity.CENTER
                    setPadding(0, ui.dp(8), 0, 0)
                })
            }, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
        }
    }

    private fun layoutNote(layout: Settings.Layout) = when (layout) {
        Settings.Layout.NINE_KEY -> R.string.layout_nine_note
        else -> R.string.layout_english_note
    }

    /** 选了双拼才有方案与 preedit 写法，选了五笔才有混输（与 PC 设置程序的这几项一致）。 */
    private fun schemeRows(settings: Settings): List<View> = when (settings.layout) {
        Settings.Layout.SHUANGPIN -> {
            val choices = Shuangpin.choices
            val selected = choices.indexOfFirst { it.key == settings.shuangpinScheme }.coerceAtLeast(0)
            listOf(
                ui.picker(R.string.shuangpin_scheme, choices.getOrNull(selected)?.label.orEmpty(), choices.map { it.label }, selected) {
                    prefs.edit().putString(Settings.KEY_SHUANGPIN, choices[it].key).apply()
                    rebuild()
                },
                ui.toggle(R.string.shuangpin_raw_title, R.string.shuangpin_raw_note, settings.shuangpinRawPreedit) {
                    prefs.edit().putBoolean(Settings.KEY_SHUANGPIN_RAW, it).apply()
                },
                ui.toggle(R.string.shuangpin_hints_title, R.string.shuangpin_hints_note, settings.shuangpinHints) {
                    prefs.edit().putBoolean(Settings.KEY_SHUANGPIN_HINTS, it).apply()
                },
            )
        }

        Settings.Layout.WUBI -> listOf(
            ui.toggle(R.string.wubi_mixed_title, R.string.wubi_mixed_note, settings.wubiMixed) {
                prefs.edit().putBoolean(Settings.KEY_WUBI_MIXED, it).apply()
            },
        )

        else -> emptyList()
    }

    private fun fuzzyPicker(settings: Settings): View {
        val fuzzy = SettingChoices.fuzzy
        val checked = BooleanArray(fuzzy.size) { fuzzy[it].key in settings.inputOptions.fuzzy }
        val count = checked.count { it }
        return ui.multiPicker(
            R.string.fuzzy_title,
            if (count == 0) getString(R.string.fuzzy_off) else getString(R.string.fuzzy_on, count),
            fuzzy.map { it.label }, checked,
        ) { picked ->
            prefs.edit().putStringSet(Settings.KEY_FUZZY, fuzzy.filterIndexed { i, _ -> picked[i] }.map { it.key }.toSet()).apply()
            rebuild()
        }
    }

    /** 打拼音给英文词（开着才有中文排前面）、英文键盘给候选。 */
    private fun mixedRows(settings: Settings): List<View> {
        val options = settings.inputOptions
        val rows = mutableListOf<View>(
            ui.toggle(R.string.english_in_chinese_title, null, options.englishInChinese) {
                prefs.edit().putBoolean(Settings.KEY_ENGLISH_IN_CHINESE, it).apply()
                rebuild()
            },
        )
        if (options.englishInChinese) {
            rows += ui.toggle(R.string.chinese_first_title, null, options.chineseFirst) {
                prefs.edit().putBoolean(Settings.KEY_CHINESE_FIRST, it).apply()
            }
        }
        rows += ui.toggle(R.string.english_candidates_title, R.string.english_candidates_note, settings.englishCandidates) {
            prefs.edit().putBoolean(Settings.KEY_ENGLISH_CANDIDATES, it).apply()
        }
        return rows
    }

    /** 不显示 / 1 / 2 / 3：一次最多配几个，同 PC 设置程序「候选后面」。 */
    private fun extraRow(title: Int, key: String, current: Int): View {
        val counts = SettingChoices.extraCounts
        val labels = counts.map { if (it == 0) getString(R.string.extras_off) else it.toString() }
        return ui.segmentedRow(title, labels, counts.indexOf(current).coerceAtLeast(0)) {
            prefs.edit().putInt(key, counts[it]).apply()
            rebuild()
        }
    }

    private fun variantPicker(settings: Settings): View {
        val variants = SettingChoices.traditionalVariants
        val current = settings.inputOptions.traditionalVariant
        return ui.picker(
            R.string.traditional_variant_title, SettingChoices.traditionalVariantName(current),
            variants.map { it.label }, variants.indexOfFirst { it.key == current },
        ) {
            prefs.edit().putString(Settings.KEY_TRADITIONAL_VARIANT, variants[it].key).apply()
            rebuild()
        }
    }

    companion object {
        /** 方案与方块上的字。 */
        private val LAYOUTS = listOf(
            Settings.Layout.QWERTY to "QW",
            Settings.Layout.NINE_KEY to "9",
            Settings.Layout.SHUANGPIN to "双",
            Settings.Layout.WUBI to "五",
            Settings.Layout.ZHUYIN to "ㄅ",
        )
    }
}
