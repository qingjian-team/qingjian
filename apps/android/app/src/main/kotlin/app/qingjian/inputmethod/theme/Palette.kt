package app.qingjian.inputmethod.theme

import android.content.Context
import app.qingjian.inputmethod.core.Native
import app.qingjian.inputmethod.settings.Settings
import org.json.JSONObject

/**
 * 键盘、面板与浮动编辑条的配色。由 Rust 侧按设置里的主题推出来（`qingjian-render` 的 `KeyboardColors`），
 * 与候选条同一个主题、同一份规则，这里不写颜色。深浅色按「外观」设置，主题锁定了外观时以主题为准。
 */
class Palette(
    /** 实际是深色（锁定浅色的主题在深色模式下也是 false）；候选条按它画。 */
    val dark: Boolean,
    /** 键盘底板。 */
    val board: Int,
    /** 字母键与候选条的面。 */
    val surface: Int,
    val functionalKey: Int,
    /** 按下与首选高亮，对应桌面候选窗的高亮条。 */
    val accent: Int,
    /** 回车键与品牌色。 */
    val brand: Int,
    val onBrand: Int,
    val keyShadow: Int,
    val text: Int,
    val gloss: Int,
    val faint: Int,
    val fresh: Int,
    /** 云联想的词与整句（主题的 `cloud`）。 */
    val cloud: Int,
) {
    companion object {
        /** 推过的配色，按（主题, 深浅）记着：键盘每次重建都要取，不必每次过 JNI。 */
        private val cache = mutableMapOf<Pair<String, Boolean>, Palette>()

        /** 当前设置下的配色。 */
        fun of(context: Context): Palette {
            val settings = Settings.load(context)
            return of(settings.theme, settings.appearance.dark(context))
        }

        /** 主题 [theme] 在 [dark] 外观下的配色。 */
        fun of(theme: String, dark: Boolean): Palette = cache.getOrPut(theme to dark) {
            parse(JSONObject(Native.keyboardPalette(theme, dark) ?: error("推不出键盘配色：$theme")))
        }

        private fun parse(json: JSONObject): Palette {
            fun color(name: String) = json.getLong(name).toInt()
            return Palette(
                dark = json.getBoolean("dark"),
                board = color("board"),
                surface = color("key"),
                functionalKey = color("function_key"),
                accent = color("pressed"),
                brand = color("brand"),
                onBrand = color("on_brand"),
                keyShadow = color("shadow"),
                text = color("text"),
                gloss = color("gloss"),
                faint = color("faint"),
                fresh = color("fresh"),
                cloud = color("cloud"),
            )
        }
    }
}
