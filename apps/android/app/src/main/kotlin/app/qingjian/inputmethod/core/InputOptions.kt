package app.qingjian.inputmethod.core

import app.qingjian.inputmethod.core.store.SettingChoices
import org.json.JSONArray
import org.json.JSONObject

/**
 * 交给 Rust 的几项键盘选项：模糊音、中文标点全半角、打拼音时给不给英文词、学习语言、繁体字形、学习开关、附加词库、分次选词、拼写纠错、
 * 候选后面配几个 emoji / 颜文字 / 符号。
 * 名字同 PC 配置文件（`[fuzzy]`、`[general]`、`[dictionaries]`），对应 Rust 的 `InputOptions`；可选的值与缺省值见 [SettingChoices]。
 */
data class InputOptions(
    val fuzzy: Set<String> = emptySet(),
    val fullWidthPunctuation: Boolean = true,
    val englishInChinese: Boolean = true,
    val chineseFirst: Boolean = false,
    val learningLanguage: String = SettingChoices.defaultLearningLanguage,
    /** 功能面板「繁体」开着时的字形：`taiwan` / `hongkong` / `standard`。 */
    val traditionalVariant: String = SettingChoices.defaultTraditionalVariant,
    /** 学习输入习惯：关了不再记新的，已学的照常参与排序。 */
    val learning: Boolean = true,
    /** 打开的随包领域词库（文件名不含扩展名）。 */
    val domains: Set<String> = SettingChoices.defaultDomains,
    /** 关掉的导入词库。 */
    val disabledDictionaries: Set<String> = emptySet(),
    /** 一段拼音分几次选时，选好的词先留在组句里、整段选完才上屏；退格先撤回上一次选择。 */
    val holdSelections: Boolean = true,
    /** 拼写纠错（PC 的 `[general] spelling_correction`）：关了拼音只按敲的原样与模糊音读。 */
    val spellingCorrection: Boolean = true,
    /** 候选后面一次最多配几个 emoji（0 不配），PC 的 `[general] emoji`；颜文字、符号同理。 */
    val emoji: Int = SettingChoices.defaultEmoji,
    val kaomoji: Int = SettingChoices.defaultKaomoji,
    val symbols: Int = SettingChoices.defaultSymbols,
    /** 词库文件变了（导入、删除）就换一个值，好让选项不相等、重新交给会话；不进 JSON。 */
    val dictionariesStamp: Long = 0,
) {
    fun toJson(): String = JSONObject()
        .put("fuzzy", JSONArray(SettingChoices.fuzzy.map { it.key }.filter { it in fuzzy }))
        .put("full_width_punctuation", fullWidthPunctuation)
        .put("english_in_chinese", englishInChinese)
        .put("chinese_first", chineseFirst)
        .put("learning_language", learningLanguage)
        .put("traditional_variant", traditionalVariant)
        .put("learning", learning)
        .put("domains", JSONArray(domains.sorted()))
        .put("disabled_dictionaries", JSONArray(disabledDictionaries.sorted()))
        .put("hold_selections", holdSelections)
        .put("spelling_correction", spellingCorrection)
        .put("emoji", emoji)
        .put("kaomoji", kaomoji)
        .put("symbols", symbols)
        .toString()

    companion object {
        const val LEARNING_OFF = "off"
    }
}
