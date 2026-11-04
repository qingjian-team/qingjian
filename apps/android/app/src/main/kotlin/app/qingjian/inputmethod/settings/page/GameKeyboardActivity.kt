package app.qingjian.inputmethod.settings.page

import android.view.Gravity
import android.view.View
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.settings.GameKeyboard

/**
 * 键盘 › 游戏键盘：总开关、样子（透明度、大小、译词）与用它的应用。
 * 应用只列横屏用青简打过字的（服务记下的 [GameKeyboard.seen]），勾上的那些横屏时换成悬浮小键盘。
 */
class GameKeyboardActivity : PageActivity() {
    override val title = R.string.page_game_keyboard

    override fun content(): List<View> {
        val game = GameKeyboard.load(this)
        return listOf(
            ui.group(
                null,
                ui.toggle(R.string.page_game_keyboard, null, game.enabled) {
                    prefs.edit().putBoolean(GameKeyboard.KEY_ENABLED, it).apply()
                },
                footer = R.string.game_keyboard_note,
            ),
            ui.group(
                R.string.section_game_look,
                ui.slider(
                    R.string.game_transparency_title, null, 0, GameKeyboard.TRANSPARENCY_MAX, game.transparency, step = 10,
                    format = { "$it%" },
                ) { prefs.edit().putInt(GameKeyboard.KEY_TRANSPARENCY, it).apply() },
                ui.segmentedRow(R.string.game_size_title, GameKeyboard.Size.entries.map { getString(it.label) }, game.size.ordinal) {
                    prefs.edit().putString(GameKeyboard.KEY_SIZE, GameKeyboard.Size.entries[it].name).apply()
                },
                ui.toggle(R.string.game_glosses_title, null, game.glosses) {
                    prefs.edit().putBoolean(GameKeyboard.KEY_GLOSSES, it).apply()
                },
                footer = R.string.game_look_note,
            ),
            ui.group(
                getString(R.string.section_game_apps),
                appRows(game),
                getString(R.string.game_apps_note),
            ),
        )
    }

    // 透明度、勾选在游戏里也能改（拖动栏、功能面板），回到这页重画
    override fun onRestart() {
        super.onRestart()
        rebuild()
    }

    private fun appRows(game: GameKeyboard): List<View> {
        val seen = GameKeyboard.seen(this)
        if (seen.isEmpty()) return listOf(ui.row(getString(R.string.game_apps_empty), null))
        return seen.map { app ->
            ui.toggle(app.label, null, app.packageName in game.apps, null) {
                GameKeyboard.setApp(this, app.packageName, it)
            }.apply {
                addView(appIcon(app), 0, LinearLayout.LayoutParams(ui.dp(ICON_DP), ui.dp(ICON_DP)).apply { marginStart = ui.dp(16) })
                getChildAt(1).setPadding(ui.dp(12), ui.dp(12), ui.dp(8), ui.dp(12))
            }
        }
    }

    /** 应用图标；取不到（应用卸了、查不到）就用名字的头一个字放在色块里。 */
    private fun appIcon(app: GameKeyboard.SeenApp): View {
        val icon = runCatching { packageManager.getApplicationIcon(app.packageName) }.getOrNull()
        if (icon != null) return ImageView(this).apply { setImageDrawable(icon) }
        return TextView(this).apply {
            text = app.label.take(1)
            gravity = Gravity.CENTER
            textSize = 13f
            setTextColor(0xFFFFFFFF.toInt())
            background = ui.rounded(ui.colors.tileMuted, ICON_DP * 0.25f)
        }
    }

    private companion object {
        const val ICON_DP = 32
    }
}
