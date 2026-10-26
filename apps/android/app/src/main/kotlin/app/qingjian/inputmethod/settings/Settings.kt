package app.qingjian.inputmethod.settings

import android.content.Context
import android.content.SharedPreferences
import app.qingjian.inputmethod.core.InputOptions
import app.qingjian.inputmethod.core.store.SettingChoices
import app.qingjian.inputmethod.theme.Appearance
import app.qingjian.inputmethod.theme.TextSize

/** 键盘设置的一份快照；键盘每次重建时读一次。 */
data class Settings(
    val layout: Layout,
    /** 键盘高度，相对缺省行高的百分比。 */
    val heightPercent: Int,
    /** 全面屏适配：手势条那一截不留白，左右两个角放 [bottomLeft] / [bottomRight]。 */
    val fullscreenAdapt: Boolean,
    val bottomLeft: BottomAction,
    val bottomRight: BottomAction,
    val haptic: Boolean,
    val keyPopup: Boolean,
    /** 候选出繁体（字形按 [InputOptions.traditionalVariant]）。 */
    val traditional: Boolean,
    /** 上次用的语言与中英键的另一边（[app.qingjian.inputmethod.service.input.Language] 的名字），没记过为 null。 */
    val language: String?,
    val partner: String?,
    /** 长按挑语言时另一边固定中文。 */
    val keepChinese: Boolean,
    /** 按住空格不动进语音（缺省开）；关了长按空格只挪光标。 */
    val voiceHoldSpace: Boolean,
    /** 全键盘拼音与英文键盘上手指划过字母连成一串（缺省关）。 */
    val swipeInput: Boolean,
    /** 双拼方案（Core 的方案名，同 PC 的 `[general] shuangpin`），键盘选「双拼」时用。 */
    val shuangpinScheme: String,
    /** 双拼的 preedit 显示敲的键，不展开成全拼（同 PC 的 `shuangpin_raw_preedit`）。 */
    val shuangpinRawPreedit: Boolean,
    /** 五笔与拼音混输（同 PC 五笔、拼音两边都开）。 */
    val wubiMixed: Boolean,
    /** 记剪贴板历史（复制的文字、图片存在本机）。 */
    val recordClipboard: Boolean,
    /** 剪贴板最多存多少条，超了丢最旧的。 */
    val clipboardLimit: Int,
    /** 剪贴板里像代码的文字按关键字、字符串、注释、数字上色。 */
    val codeHighlight: Boolean,
    /** 模糊音、中文标点全半角、打拼音时给不给英文词、学习语言、繁体字形、学习开关、附加词库（交给 Core）。 */
    val inputOptions: InputOptions,
    /** 英文键盘也组句给英文词候选；关了字母直接上屏。 */
    val englishCandidates: Boolean,
    /** 补充译词：随包释义表查不到译词的词上屏后问云端补写（缺省关，要先配好云端）。 */
    val glossFill: Boolean,
    /** 云联想与整句补全（缺省关，要先配好云端）。 */
    val cloudPredict: Boolean,
    /** 云联想只在 Wi-Fi（含有线）下发（缺省开）；快捷翻译与补充译词不受限。 */
    val cloudWifiOnly: Boolean,
    /** 键盘、候选条与面板用的主题（内置主题 id，同 PC 的 `[general] theme`）。 */
    val theme: String,
    val appearance: Appearance,
    /** 候选字号档。 */
    val textSize: TextSize,
) {
    enum class Layout { QWERTY, NINE_KEY, ZHUYIN, SHUANGPIN, WUBI }

    companion object {
        const val KEY_LAYOUT = "layout"
        const val KEY_HEIGHT = "height_percent"
        const val KEY_FULLSCREEN = "fullscreen_adapt"
        const val KEY_BOTTOM_LEFT = "bottom_left"
        const val KEY_BOTTOM_RIGHT = "bottom_right"
        const val KEY_HAPTIC = "haptic"
        const val KEY_POPUP = "key_popup"
        const val KEY_TRADITIONAL = "traditional"
        const val KEY_LANGUAGE = "language"
        const val KEY_PARTNER = "language_partner"
        const val KEY_KEEP_CHINESE = "keep_chinese"
        const val KEY_VOICE_HOLD_SPACE = "voice_hold_space"
        const val KEY_SWIPE_INPUT = "swipe_input"
        const val KEY_SHUANGPIN = "shuangpin_scheme"
        const val KEY_SHUANGPIN_RAW = "shuangpin_raw_preedit"
        const val KEY_WUBI_MIXED = "wubi_mixed"
        const val KEY_RECORD_CLIPBOARD = "record_clipboard"
        const val KEY_CLIPBOARD_LIMIT = "clipboard_limit"
        const val KEY_CODE_HIGHLIGHT = "code_highlight"
        const val KEY_FUZZY = "fuzzy"
        const val KEY_FULL_WIDTH = "full_width_punctuation"
        const val KEY_ENGLISH_IN_CHINESE = "english_in_chinese"
        const val KEY_CHINESE_FIRST = "chinese_first"
        const val KEY_ENGLISH_CANDIDATES = "english_candidates"
        const val KEY_LEARNING_LANGUAGE = "learning_language"
        const val KEY_TRADITIONAL_VARIANT = "traditional_variant"
        const val KEY_LEARNING = "learning"
        const val KEY_HOLD_SELECTIONS = "hold_selections"
        const val KEY_DOMAINS = "domains"
        const val KEY_DISABLED_DICTIONARIES = "disabled_dictionaries"
        const val KEY_GLOSS_FILL = "gloss_fill"
        const val KEY_CLOUD_PREDICT = "cloud_predict"
        const val KEY_CLOUD_WIFI_ONLY = "cloud_wifi_only"
        const val KEY_THEME = "theme"
        const val KEY_APPEARANCE = "appearance"
        const val KEY_TEXT_SIZE = "text_size"

        /** 没选过时的主题，同 PC 的缺省主题。 */
        const val DEFAULT_THEME = "qingjian"

        /** 导入或删了词库就改它，键盘重建时交设置、会话按文件重挂。 */
        const val KEY_DICTIONARIES_STAMP = "dictionaries_stamp"

        /** 没选过时的双拼方案，与 PC 设置程序列表的第一个一致。 */
        const val DEFAULT_SHUANGPIN = "xiaohe"

        const val HEIGHT_MIN = 70
        const val HEIGHT_MAX = 150

        fun preferences(context: Context): SharedPreferences =
            context.getSharedPreferences("settings", Context.MODE_PRIVATE)

        /** 学习语言的代码（`en`…，`off` 不显示译词）；App 的生词本与统计页也按它。 */
        fun learningLanguage(context: Context): String =
            preferences(context).getString(KEY_LEARNING_LANGUAGE, null) ?: SettingChoices.defaultLearningLanguage

        fun load(context: Context): Settings {
            val prefs = preferences(context)
            return Settings(
                layout = Layout.entries.firstOrNull { it.name == prefs.getString(KEY_LAYOUT, null) }
                    ?: Layout.QWERTY,
                heightPercent = prefs.getInt(KEY_HEIGHT, 100).coerceIn(HEIGHT_MIN, HEIGHT_MAX),
                fullscreenAdapt = prefs.getBoolean(KEY_FULLSCREEN, true),
                bottomLeft = BottomAction.of(prefs.getString(KEY_BOTTOM_LEFT, null), BottomAction.SWITCH_INPUT_METHOD),
                bottomRight = BottomAction.of(prefs.getString(KEY_BOTTOM_RIGHT, null), BottomAction.VOICE),
                haptic = prefs.getBoolean(KEY_HAPTIC, true),
                keyPopup = prefs.getBoolean(KEY_POPUP, true),
                traditional = prefs.getBoolean(KEY_TRADITIONAL, false),
                language = prefs.getString(KEY_LANGUAGE, null),
                partner = prefs.getString(KEY_PARTNER, null),
                keepChinese = prefs.getBoolean(KEY_KEEP_CHINESE, true),
                voiceHoldSpace = prefs.getBoolean(KEY_VOICE_HOLD_SPACE, true),
                swipeInput = prefs.getBoolean(KEY_SWIPE_INPUT, false),
                shuangpinScheme = prefs.getString(KEY_SHUANGPIN, null) ?: DEFAULT_SHUANGPIN,
                shuangpinRawPreedit = prefs.getBoolean(KEY_SHUANGPIN_RAW, false),
                wubiMixed = prefs.getBoolean(KEY_WUBI_MIXED, false),
                recordClipboard = prefs.getBoolean(KEY_RECORD_CLIPBOARD, true),
                clipboardLimit = prefs.getInt(KEY_CLIPBOARD_LIMIT, SettingChoices.defaultClipboardLimit).coerceAtLeast(1),
                codeHighlight = prefs.getBoolean(KEY_CODE_HIGHLIGHT, true),
                inputOptions = InputOptions(
                    fuzzy = prefs.getStringSet(KEY_FUZZY, null).orEmpty().toSet(),
                    fullWidthPunctuation = prefs.getBoolean(KEY_FULL_WIDTH, true),
                    englishInChinese = prefs.getBoolean(KEY_ENGLISH_IN_CHINESE, true),
                    chineseFirst = prefs.getBoolean(KEY_CHINESE_FIRST, false),
                    learningLanguage = learningLanguage(context),
                    traditionalVariant = prefs.getString(KEY_TRADITIONAL_VARIANT, null) ?: SettingChoices.defaultTraditionalVariant,
                    learning = prefs.getBoolean(KEY_LEARNING, true),
                    holdSelections = prefs.getBoolean(KEY_HOLD_SELECTIONS, true),
                    domains = prefs.getStringSet(KEY_DOMAINS, null)?.toSet() ?: SettingChoices.defaultDomains,
                    disabledDictionaries = prefs.getStringSet(KEY_DISABLED_DICTIONARIES, null).orEmpty().toSet(),
                    dictionariesStamp = prefs.getLong(KEY_DICTIONARIES_STAMP, 0),
                ),
                englishCandidates = prefs.getBoolean(KEY_ENGLISH_CANDIDATES, true),
                glossFill = prefs.getBoolean(KEY_GLOSS_FILL, false),
                cloudPredict = prefs.getBoolean(KEY_CLOUD_PREDICT, false),
                cloudWifiOnly = prefs.getBoolean(KEY_CLOUD_WIFI_ONLY, true),
                theme = prefs.getString(KEY_THEME, null) ?: DEFAULT_THEME,
                appearance = Appearance.of(prefs.getString(KEY_APPEARANCE, null)),
                textSize = TextSize.of(prefs.getString(KEY_TEXT_SIZE, null)),
            )
        }
    }
}
