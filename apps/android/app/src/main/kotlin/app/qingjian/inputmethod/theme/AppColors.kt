package app.qingjian.inputmethod.theme

import android.content.Context
import android.content.res.Configuration

/**
 * App 页面的配色，与键盘的 [Palette] 分开：中性浅灰底、纯白卡片、
 * 页面顶上一层淡青绿渐变；品牌色只用在选中、按钮与数字标签上，功能入口是一组彩色图标块。跟随系统深浅色。
 */
class AppColors(
    val background: Int,
    /** 页面顶部渐变的起始色，往下渐变到 [background]。 */
    val wash: Int,
    val surface: Int,
    val text: Int,
    val secondary: Int,
    val faint: Int,
    val divider: Int,
    val brand: Int,
    val onBrand: Int,
    /** 品牌色的浅底：选中的单选外圈、标签底。 */
    val brandSoft: Int,
    /** 功能入口图标块的颜色，按顺序取。 */
    val tiles: List<Int>,
    val dark: Boolean,
) {
    companion object {
        fun of(context: Context): AppColors {
            val night = context.resources.configuration.uiMode and
                Configuration.UI_MODE_NIGHT_MASK == Configuration.UI_MODE_NIGHT_YES
            return if (night) DARK else LIGHT
        }

        const val JADE = 0
        const val BLUE = 1
        const val ORANGE = 2
        const val PURPLE = 3
        const val PINK = 4
        const val CYAN = 5

        private val TILES = listOf(
            0xFF2FAE6A.toInt(),
            0xFF4A8CF7.toInt(),
            0xFFFF9443.toInt(),
            0xFF8D6EF2.toInt(),
            0xFFF16C9A.toInt(),
            0xFF23B5C3.toInt(),
        )

        private val LIGHT = AppColors(
            background = 0xFFF4F6F5.toInt(),
            wash = 0xFFD9F1E2.toInt(),
            surface = 0xFFFFFFFF.toInt(),
            text = 0xFF16191A.toInt(),
            secondary = 0xFF8A9093.toInt(),
            faint = 0xFFC3C8CB.toInt(),
            divider = 0xFFEEF1F0.toInt(),
            brand = 0xFF239A58.toInt(),
            onBrand = 0xFFFFFFFF.toInt(),
            brandSoft = 0xFFE3F4EA.toInt(),
            tiles = TILES,
            dark = false,
        )

        private val DARK = AppColors(
            background = 0xFF0E1110.toInt(),
            wash = 0xFF143223.toInt(),
            surface = 0xFF1A1E1C.toInt(),
            text = 0xFFEDF0EE.toInt(),
            secondary = 0xFF8E9693.toInt(),
            faint = 0xFF4A514E.toInt(),
            divider = 0xFF252A28.toInt(),
            brand = 0xFF45C47D.toInt(),
            onBrand = 0xFF08130C.toInt(),
            brandSoft = 0xFF1D3628.toInt(),
            tiles = TILES,
            dark = true,
        )
    }
}
