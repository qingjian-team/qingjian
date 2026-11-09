package app.qingjian.inputmethod.settings

import android.content.Context
import android.content.res.Configuration
import app.qingjian.inputmethod.R

/**
 * 游戏键盘的设置：横屏时在勾上的应用里换成半透明的悬浮小键盘（[app.qingjian.inputmethod.service.GameFrame]）。
 * 与 [Settings] 同一个 SharedPreferences 文件，设置页改了开着的键盘照样重建；位置与见过的应用由服务自己写。
 */
data class GameKeyboard(
    /** 总开关；关了勾着的应用也用普通键盘。 */
    val enabled: Boolean,
    /** 透明度百分比，0 不透明。 */
    val transparency: Int,
    val size: Size,
    /** 候选旁边显示译词（缺省关：游戏里屏幕小，只要字）。 */
    val glosses: Boolean,
    /** 用游戏键盘的应用（包名）。 */
    val apps: Set<String>,
) {
    /** 键盘大小：[widthFraction] 是占屏幕宽的比例，[heightPercent] 是行高（同 [Settings.heightPercent]）。 */
    enum class Size(val label: Int, val widthFraction: Float, val heightPercent: Int) {
        SMALL(R.string.game_size_small, 0.42f, 62),
        MEDIUM(R.string.game_size_medium, 0.5f, 70),
        LARGE(R.string.game_size_large, 0.6f, 82),
    }

    /** 横屏打过字的一个应用：包名与当时取到的名字。 */
    data class SeenApp(val packageName: String, val label: String)

    companion object {
        const val KEY_ENABLED = "game_keyboard"
        const val KEY_TRANSPARENCY = "game_transparency"
        const val KEY_SIZE = "game_size"
        const val KEY_GLOSSES = "game_glosses"
        const val KEY_APPS = "game_apps"
        private const val KEY_SEEN = "game_seen"
        private const val KEY_POSITION_PREFIX = "game_position:"

        const val TRANSPARENCY_MAX = 70
        private const val DEFAULT_TRANSPARENCY = 40

        /** 见过的应用最多记这么多，多了丢最早的。 */
        private const val SEEN_LIMIT = 60

        fun load(context: Context): GameKeyboard {
            val prefs = Settings.preferences(context)
            return GameKeyboard(
                enabled = prefs.getBoolean(KEY_ENABLED, true),
                transparency = prefs.getInt(KEY_TRANSPARENCY, DEFAULT_TRANSPARENCY).coerceIn(0, TRANSPARENCY_MAX),
                size = Size.entries.firstOrNull { it.name == prefs.getString(KEY_SIZE, null) } ?: Size.MEDIUM,
                glosses = prefs.getBoolean(KEY_GLOSSES, false),
                apps = prefs.getStringSet(KEY_APPS, null).orEmpty().toSet(),
            )
        }

        /** 现在给 [packageName] 用游戏键盘吗：横屏、总开关开着、这个应用勾上了。 */
        fun wanted(context: Context, packageName: String?): Boolean {
            if (packageName == null) return false
            if (context.resources.configuration.orientation != Configuration.ORIENTATION_LANDSCAPE) return false
            val game = load(context)
            return game.enabled && packageName in game.apps
        }

        fun setApp(context: Context, packageName: String, on: Boolean) {
            val prefs = Settings.preferences(context)
            val apps = prefs.getStringSet(KEY_APPS, null).orEmpty().toMutableSet()
            if (on) apps += packageName else apps -= packageName
            prefs.edit().putStringSet(KEY_APPS, apps).apply()
        }

        /** 横屏打过字的应用，最近的在前。一行一个「包名\t名字」，按时间先后排。 */
        fun seen(context: Context): List<SeenApp> =
            Settings.preferences(context).getString(KEY_SEEN, null).orEmpty().lines()
                .mapNotNull { line -> line.split('\t').takeIf { it.size == 2 }?.let { SeenApp(it[0], it[1]) } }
                .reversed()

        /** 记下横屏打过字的 [app]；已经记过返回 false，不写。 */
        fun remember(context: Context, app: SeenApp): Boolean {
            val list = seen(context).reversed()
            if (list.any { it.packageName == app.packageName }) return false
            val kept = (list + app).takeLast(SEEN_LIMIT)
            Settings.preferences(context).edit()
                .putString(KEY_SEEN, kept.joinToString("\n") { "${it.packageName}\t${it.label.replace('\t', ' ').replace('\n', ' ')}" })
                .apply()
            return true
        }

        /** [packageName] 里键盘的位置：左上角在可挪范围里的比例（0..1）；没记过为 null。 */
        fun position(context: Context, packageName: String): Pair<Float, Float>? {
            val value = Settings.preferences(context).getString(KEY_POSITION_PREFIX + packageName, null) ?: return null
            val parts = value.split(',').mapNotNull(String::toFloatOrNull)
            return if (parts.size == 2) parts[0].coerceIn(0f, 1f) to parts[1].coerceIn(0f, 1f) else null
        }

        fun savePosition(context: Context, packageName: String, x: Float, y: Float) {
            Settings.preferences(context).edit().putString(KEY_POSITION_PREFIX + packageName, "$x,$y").commit()
        }
    }
}
