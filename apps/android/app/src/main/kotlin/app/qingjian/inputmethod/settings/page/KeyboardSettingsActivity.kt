package app.qingjian.inputmethod.settings.page

import android.content.Intent
import android.view.View
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.cloud.CloudConfig
import app.qingjian.inputmethod.cloud.SecretStore
import app.qingjian.inputmethod.core.store.SettingChoices
import app.qingjian.inputmethod.core.Native
import app.qingjian.inputmethod.core.store.Shuangpin
import app.qingjian.inputmethod.settings.BottomAction
import app.qingjian.inputmethod.settings.Settings
import kotlin.concurrent.thread
import org.json.JSONObject

/** 键盘设置二级页：拼音键盘布局、学习语言与学习开关、模糊音与标点、繁体字形、词库入口、键盘高度、全面屏适配、按键反馈、长按空格说话与滑行输入、剪贴板、云端大模型。改了马上写设置，开着的键盘跟着重建。 */
class KeyboardSettingsActivity : PageActivity() {
    override val title = R.string.page_keyboard

    override fun content(): List<View> {
        val settings = Settings.load(this)
        return listOf(
            ui.sectionTitle(R.string.section_layout),
            ui.card(
                ui.choice(R.string.layout_qwerty, settings.layout == Settings.Layout.QWERTY) {
                    setLayout(Settings.Layout.QWERTY)
                },
                ui.choice(R.string.layout_nine, settings.layout == Settings.Layout.NINE_KEY) {
                    setLayout(Settings.Layout.NINE_KEY)
                },
                ui.choice(R.string.layout_shuangpin, settings.layout == Settings.Layout.SHUANGPIN) {
                    setLayout(Settings.Layout.SHUANGPIN)
                },
                ui.choice(R.string.layout_wubi, settings.layout == Settings.Layout.WUBI) {
                    setLayout(Settings.Layout.WUBI)
                },
                ui.choice(R.string.layout_zhuyin, settings.layout == Settings.Layout.ZHUYIN) {
                    setLayout(Settings.Layout.ZHUYIN)
                },
                ui.note(R.string.layout_nine_note),
            ),
            *schemeOptions(settings),
            ui.sectionTitle(R.string.section_learning),
            learningCard(settings),
            ui.sectionTitle(R.string.section_input),
            inputCard(settings),
            ui.sectionTitle(R.string.section_size),
            ui.card(
                ui.slider(
                    R.string.height_title, R.string.height_note,
                    Settings.HEIGHT_MIN, Settings.HEIGHT_MAX, settings.heightPercent, step = 1,
                    format = { "$it%" },
                ) { prefs.edit().putInt(Settings.KEY_HEIGHT, it).apply() },
                ui.divider(),
                ui.toggle(R.string.fullscreen_title, R.string.fullscreen_note, settings.fullscreenAdapt) {
                    prefs.edit().putBoolean(Settings.KEY_FULLSCREEN, it).apply()
                    rebuild()
                },
                *bottomPickers(settings),
            ),
            ui.sectionTitle(R.string.section_keys),
            ui.card(
                ui.toggle(R.string.haptic_title, null, settings.haptic) {
                    prefs.edit().putBoolean(Settings.KEY_HAPTIC, it).apply()
                },
                ui.divider(),
                ui.toggle(R.string.popup_title, R.string.popup_note, settings.keyPopup) {
                    prefs.edit().putBoolean(Settings.KEY_POPUP, it).apply()
                },
                ui.divider(),
                ui.toggle(R.string.keep_chinese_title, R.string.keep_chinese_note, settings.keepChinese) {
                    prefs.edit().putBoolean(Settings.KEY_KEEP_CHINESE, it).apply()
                },
                ui.divider(),
                ui.toggle(R.string.voice_hold_space_title, R.string.voice_hold_space_note, settings.voiceHoldSpace) {
                    prefs.edit().putBoolean(Settings.KEY_VOICE_HOLD_SPACE, it).apply()
                },
                ui.divider(),
                ui.toggle(R.string.swipe_input_title, R.string.swipe_input_note, settings.swipeInput) {
                    prefs.edit().putBoolean(Settings.KEY_SWIPE_INPUT, it).apply()
                },
            ),
            ui.sectionTitle(R.string.section_clipboard),
            ui.card(
                ui.toggle(R.string.record_clipboard_title, R.string.record_clipboard_note, settings.recordClipboard) {
                    prefs.edit().putBoolean(Settings.KEY_RECORD_CLIPBOARD, it).apply()
                },
                ui.divider(),
                clipboardLimitPicker(settings),
                ui.divider(),
                ui.toggle(R.string.code_highlight_title, R.string.code_highlight_note, settings.codeHighlight) {
                    prefs.edit().putBoolean(Settings.KEY_CODE_HIGHLIGHT, it).apply()
                },
            ),
            ui.sectionTitle(R.string.section_cloud),
            cloudCard(),
        )
    }

    /** 学习语言（PC 设置程序「候选」页的「译文」）：换了键盘立即换译词，生词本与统计页也跟着换。 */
    private fun learningCard(settings: Settings): View {
        val languages = SettingChoices.learningLanguages
        val current = settings.inputOptions.learningLanguage
        return ui.card(
            ui.picker(
                R.string.learning_language_title, SettingChoices.learningLanguageName(current),
                languages.map { it.label }, languages.indexOfFirst { it.key == current },
            ) {
                prefs.edit().putString(Settings.KEY_LEARNING_LANGUAGE, languages[it].key).apply()
                rebuild()
            },
            ui.note(R.string.learning_language_note),
            ui.divider(),
            ui.toggle(R.string.learning_title, R.string.learning_note, settings.inputOptions.learning) {
                prefs.edit().putBoolean(Settings.KEY_LEARNING, it).apply()
            },
        )
    }

    /** 模糊音、中文标点、分次选词、中英混输（与 PC 设置程序「输入」「中英文」「候选」页的同名项一致）。 */
    private fun inputCard(settings: Settings): View {
        val options = settings.inputOptions
        val fuzzy = SettingChoices.fuzzy
        val checked = BooleanArray(fuzzy.size) { fuzzy[it].key in options.fuzzy }
        val count = checked.count { it }
        val rows = mutableListOf(
            ui.multiPicker(
                R.string.fuzzy_title,
                if (count == 0) getString(R.string.fuzzy_off) else getString(R.string.fuzzy_on, count),
                fuzzy.map { it.label }, checked,
            ) { picked ->
                prefs.edit().putStringSet(Settings.KEY_FUZZY, fuzzy.filterIndexed { i, _ -> picked[i] }.map { it.key }.toSet()).apply()
                rebuild()
            },
            ui.note(R.string.fuzzy_note),
            ui.divider(),
            ui.toggle(R.string.full_width_title, R.string.full_width_note, options.fullWidthPunctuation) {
                prefs.edit().putBoolean(Settings.KEY_FULL_WIDTH, it).apply()
            },
            ui.divider(),
            ui.toggle(R.string.hold_selections_title, R.string.hold_selections_note, options.holdSelections) {
                prefs.edit().putBoolean(Settings.KEY_HOLD_SELECTIONS, it).apply()
            },
            ui.divider(),
            ui.toggle(R.string.english_in_chinese_title, R.string.english_in_chinese_note, options.englishInChinese) {
                prefs.edit().putBoolean(Settings.KEY_ENGLISH_IN_CHINESE, it).apply()
                rebuild()
            },
        )
        if (options.englishInChinese) {
            rows += ui.divider()
            rows += ui.toggle(R.string.chinese_first_title, null, options.chineseFirst) {
                prefs.edit().putBoolean(Settings.KEY_CHINESE_FIRST, it).apply()
            }
        }
        rows += ui.divider()
        rows += ui.toggle(R.string.english_candidates_title, R.string.english_candidates_note, settings.englishCandidates) {
            prefs.edit().putBoolean(Settings.KEY_ENGLISH_CANDIDATES, it).apply()
        }
        val variants = SettingChoices.traditionalVariants
        rows += ui.divider()
        rows += ui.picker(
            R.string.traditional_variant_title, SettingChoices.traditionalVariantName(options.traditionalVariant),
            variants.map { it.label }, variants.indexOfFirst { it.key == options.traditionalVariant },
        ) {
            prefs.edit().putString(Settings.KEY_TRADITIONAL_VARIANT, variants[it].key).apply()
            rebuild()
        }
        rows += ui.note(R.string.traditional_variant_note)
        rows += ui.divider()
        rows += ui.action(R.string.dictionaries_title, getString(R.string.dictionaries_summary)) {
            startActivity(Intent(this, DictionariesActivity::class.java))
        }
        return ui.card(*rows.toTypedArray())
    }

    /** 上次「测试连接」的结果（给用户看的一句话）；测试中是「测试中…」。 */
    private var testResult: String? = null

    /** 云端大模型：地址、模型、密钥（Keystore 加密存），测试连接与清除密钥。 */
    private fun cloudCard(): View {
        val cloud = CloudConfig.load(this)
        val rows = mutableListOf(
            ui.input(R.string.cloud_base_url, cloud.baseUrl, cloud.baseUrl, secret = false) {
                CloudConfig.set(this, CloudConfig.KEY_BASE_URL, it.ifEmpty { CloudConfig.DEFAULT_BASE_URL })
                testResult = null
                rebuild()
            },
            ui.divider(),
            ui.input(R.string.cloud_model, cloud.model, cloud.model, secret = false) {
                CloudConfig.set(this, CloudConfig.KEY_MODEL, it.ifEmpty { CloudConfig.DEFAULT_MODEL })
                testResult = null
                rebuild()
            },
            ui.divider(),
            ui.input(R.string.cloud_key, cloud.keyHint ?: getString(R.string.cloud_key_empty), "", secret = true) {
                if (it.isNotEmpty()) SecretStore.save(this, it)
                testResult = null
                rebuild()
            },
            ui.divider(),
            ui.action(R.string.cloud_test, testResult) { testConnection(cloud) },
        )
        if (cloud.keyHint != null) {
            rows += ui.divider()
            rows += ui.action(R.string.cloud_clear_key, null) {
                SecretStore.clear(this)
                testResult = null
                rebuild()
            }
        }
        rows += ui.note(R.string.cloud_note)
        rows += ui.divider()
        // 没配好云端时开关摆着也不生效，换成一行说明
        val settings = Settings.load(this)
        if (cloud.ready) {
            rows += ui.toggle(R.string.cloud_predict_title, R.string.cloud_predict_note, settings.cloudPredict) {
                prefs.edit().putBoolean(Settings.KEY_CLOUD_PREDICT, it).apply()
                rebuild()
            }
            if (settings.cloudPredict) {
                rows += ui.divider()
                rows += ui.toggle(R.string.cloud_wifi_only_title, R.string.cloud_wifi_only_note, settings.cloudWifiOnly) {
                    prefs.edit().putBoolean(Settings.KEY_CLOUD_WIFI_ONLY, it).apply()
                }
            }
            rows += ui.divider()
            rows += ui.toggle(R.string.gloss_fill_title, R.string.gloss_fill_note, settings.glossFill) {
                prefs.edit().putBoolean(Settings.KEY_GLOSS_FILL, it).apply()
            }
        } else {
            rows += ui.row(R.string.cloud_predict_title, R.string.gloss_fill_unavailable)
            rows += ui.divider()
            rows += ui.row(R.string.gloss_fill_title, R.string.gloss_fill_unavailable)
        }
        return ui.card(*rows.toTypedArray())
    }

    /** 测试连接要等网络，放后台线程；回来再重画这一页。 */
    private fun testConnection(cloud: CloudConfig) {
        if (testResult == getString(R.string.cloud_testing)) return
        if (!cloud.ready) {
            testResult = getString(R.string.translate_no_cloud)
            rebuild()
            return
        }
        testResult = getString(R.string.cloud_testing)
        rebuild()
        val settings = cloud.toJson()
        thread(name = "qingjian-cloud-test") {
            val outcome = runCatching { JSONObject(Native.cloudTest(settings) ?: "{}") }.getOrNull()
            val message = outcome?.optString("message")?.takeIf { it.isNotEmpty() } ?: "测试失败"
            runOnUiThread {
                if (isDestroyed) return@runOnUiThread
                testResult = message
                rebuild()
            }
        }
    }

    /** 全面屏底栏左右两个角放什么；关着全面屏适配就不显示。 */
    private fun bottomPickers(settings: Settings): Array<View> {
        if (!settings.fullscreenAdapt) return emptyArray()
        return arrayOf(
            bottomPicker(R.string.bottom_left, Settings.KEY_BOTTOM_LEFT, settings.bottomLeft, Settings.KEY_BOTTOM_RIGHT, settings.bottomRight),
            bottomPicker(R.string.bottom_right, Settings.KEY_BOTTOM_RIGHT, settings.bottomRight, Settings.KEY_BOTTOM_LEFT, settings.bottomLeft),
        )
    }

    private fun bottomPicker(title: Int, key: String, current: BottomAction, otherKey: String, other: BottomAction): View {
        val options = BottomAction.entries
        return ui.picker(title, getString(current.label), options.map { getString(it.label) }, options.indexOf(current)) { index ->
            val picked = options[index]
            val editor = prefs.edit().putString(key, picked.name)
            // 两边不能一样：选了另一边正放着的，就把这边原来的换过去
            if (picked == other && picked != BottomAction.NONE) editor.putString(otherKey, current.name)
            editor.apply()
            rebuild()
        }
    }

    /** 剪贴板最多存多少条；改小了下次记的时候丢最旧的。 */
    private fun clipboardLimitPicker(settings: Settings): View {
        val limits = SettingChoices.clipboardLimits
        val selected = limits.indexOf(settings.clipboardLimit).coerceAtLeast(0)
        return ui.picker(R.string.clipboard_limit, getString(R.string.clipboard_limit_value, limits[selected]), limits.map { getString(R.string.clipboard_limit_value, it) }, selected) {
            prefs.edit().putInt(Settings.KEY_CLIPBOARD_LIMIT, limits[it]).apply()
            rebuild()
        }
    }

    /** 选了双拼才有方案与 preedit 写法，选了五笔才有混输（与 PC 设置程序的这几项一致）。 */
    private fun schemeOptions(settings: Settings): Array<View> = when (settings.layout) {
        Settings.Layout.SHUANGPIN -> {
            val choices = Shuangpin.choices
            val selected = choices.indexOfFirst { it.key == settings.shuangpinScheme }.coerceAtLeast(0)
            arrayOf(
                ui.sectionTitle(R.string.section_shuangpin),
                ui.card(
                    ui.picker(R.string.shuangpin_scheme, choices.getOrNull(selected)?.label.orEmpty(), choices.map { it.label }, selected) {
                        prefs.edit().putString(Settings.KEY_SHUANGPIN, choices[it].key).apply()
                        rebuild()
                    },
                    ui.divider(),
                    ui.toggle(R.string.shuangpin_raw_title, R.string.shuangpin_raw_note, settings.shuangpinRawPreedit) {
                        prefs.edit().putBoolean(Settings.KEY_SHUANGPIN_RAW, it).apply()
                    },
                ),
            )
        }

        Settings.Layout.WUBI -> arrayOf(
            ui.sectionTitle(R.string.section_wubi),
            ui.card(
                ui.toggle(R.string.wubi_mixed_title, R.string.wubi_mixed_note, settings.wubiMixed) {
                    prefs.edit().putBoolean(Settings.KEY_WUBI_MIXED, it).apply()
                },
            ),
        )

        else -> emptyArray()
    }

    private fun setLayout(layout: Settings.Layout) {
        prefs.edit().putString(Settings.KEY_LAYOUT, layout.name).apply()
        rebuild()
    }
}
