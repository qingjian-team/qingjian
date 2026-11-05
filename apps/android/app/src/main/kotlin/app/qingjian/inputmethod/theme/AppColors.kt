package app.qingjian.inputmethod.theme

import android.content.Context
import android.content.res.Configuration

/**
 * App 页面的配色（「素笺」）：浅灰底、白色分组卡片、半透明的顶栏与底栏，只用一个竹青色做强调；
 * 与键盘的 [Palette] 分开；深浅色按「外观」设置（Android 12 起，见 [Appearance.applyToApp]），更早的系统跟随系统。
 */
class AppColors(
    val background: Int,
    val surface: Int,
    /** 顶栏、底部 tab 栏的底色。 */
    val bar: Int,
    val text: Int,
    val secondary: Int,
    /** 箭头、占位字这类最弱的灰。 */
    val faint: Int,
    val divider: Int,
    val brand: Int,
    val onBrand: Int,
    /** 品牌色的浅底：选中的方案块、标签底。 */
    val brandSoft: Int,
    /** 设置入口前小图标块的底色。 */
    val tile: Int,
    /** 次要入口（数据与隐私、关于）的图标块底色。 */
    val tileMuted: Int,
    /** 开关关着时的轨道、分段控件的槽。 */
    val fill: Int,
    /** 生词本「生词」标签的字色（底色是它的淡色）。 */
    val fresh: Int,
    /** 清空、清除密钥这类危险操作的字色。 */
    val danger: Int,
    val dark: Boolean,
) {
    companion object {
        fun of(context: Context): AppColors {
            val night = context.resources.configuration.uiMode and
                Configuration.UI_MODE_NIGHT_MASK == Configuration.UI_MODE_NIGHT_YES
            return if (night) DARK else LIGHT
        }

        private val LIGHT = AppColors(
            background = 0xFFF2F2F7.toInt(),
            surface = 0xFFFFFFFF.toInt(),
            bar = 0xF0F9F9F9.toInt(),
            text = 0xFF1C1C1E.toInt(),
            secondary = 0xFF6C6C70.toInt(),
            faint = 0xFFC4C4C7.toInt(),
            divider = 0xFFD8D8DC.toInt(),
            brand = 0xFF1F7A4A.toInt(),
            onBrand = 0xFFFFFFFF.toInt(),
            brandSoft = 0xFFE3F1E8.toInt(),
            tile = 0xFF1F7A4A.toInt(),
            tileMuted = 0xFF8E8E93.toInt(),
            fill = 0xFFE3E3E8.toInt(),
            fresh = 0xFFC2410C.toInt(),
            danger = 0xFFD92D35.toInt(),
            dark = false,
        )

        private val DARK = AppColors(
            background = 0xFF000000.toInt(),
            surface = 0xFF1C1C1E.toInt(),
            bar = 0xF0161618.toInt(),
            text = 0xFFF2F2F7.toInt(),
            secondary = 0xFF98989F.toInt(),
            faint = 0xFF5A5A5E.toInt(),
            divider = 0xFF38383A.toInt(),
            brand = 0xFF3FBF7A.toInt(),
            onBrand = 0xFF04150C.toInt(),
            brandSoft = 0xFF12301F.toInt(),
            tile = 0xFF2E8F5B.toInt(),
            tileMuted = 0xFF636366.toInt(),
            fill = 0xFF39393D.toInt(),
            fresh = 0xFFFF9F5A.toInt(),
            danger = 0xFFFF6369.toInt(),
            dark = true,
        )
    }
}
