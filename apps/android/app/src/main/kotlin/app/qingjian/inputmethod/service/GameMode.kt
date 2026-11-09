package app.qingjian.inputmethod.service

import android.content.Context
import android.content.res.Configuration
import android.graphics.Rect
import android.inputmethodservice.InputMethodService
import android.view.View
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.settings.GameKeyboard
import app.qingjian.inputmethod.settings.Settings
import app.qingjian.inputmethod.theme.Palette

/**
 * 游戏键盘这一块：哪个应用该用它、建浮层、触摸区，以及拖动栏上的按钮。
 * 改设置都经 [Host.writeSettings]（不触发重建键盘），要换键盘时交给 [Host.rebuild]。
 */
class GameMode(private val context: Context, private val host: Host) : GameFrame.Listener {
    interface Host {
        /** 当前输入框所在应用的包名，没有输入框时是 null。 */
        val editorPackage: String?

        /** 写设置时不重建键盘。 */
        fun writeSettings(block: () -> Unit)

        /** 重建键盘并按需进出全屏编辑。 */
        fun rebuild()

        fun hide()

        fun toast(message: String)
    }

    /** 游戏键盘开着时它浮着的那一层，与它给的应用（包名）；普通键盘时都是 null。 */
    private var frame: GameFrame? = null
    var packageName: String? = null
        private set
    private val touch = Rect()

    /** 现在的输入框该不该用游戏键盘：返回它的应用包名，不该用是 null（见 [GameKeyboard.wanted]）。 */
    fun target(): String? = host.editorPackage?.takeIf { GameKeyboard.wanted(context, it) }

    /** 这个应用登记了游戏键盘没有；没有输入框或是自己的输入框时为 null。 */
    val enabledHere: Boolean?
        get() = host.editorPackage?.takeIf { it != context.packageName }
            ?.let { it in GameKeyboard.load(context).apps }

    /** 键盘重建：[game] 不为 null 时把 [content] 包进浮层返回，否则返回 null。 */
    fun wrap(game: String?, settings: Settings, gameSettings: GameKeyboard?, content: View): View? {
        packageName = game
        if (game == null || gameSettings == null) {
            frame = null
            return null
        }
        val (x, y) = GameKeyboard.position(context, game) ?: (0.5f to 1f)
        return GameFrame(
            context, Palette.of(settings.theme, dark = true), content,
            gameSettings.size.widthFraction, gameSettings.transparency, x, y, this,
        ).also { frame = it }
    }

    /**
     * 游戏键盘铺满整屏但不顶起应用：内容区与可见区都从窗口最底下算起，只有键盘那一块收触摸，
     * 别处的点击透给游戏。普通键盘时不动。
     */
    fun computeInsets(outInsets: InputMethodService.Insets, windowHeight: Int?) {
        val frame = frame ?: return
        val height = windowHeight ?: return
        outInsets.contentTopInsets = height
        outInsets.visibleTopInsets = height
        frame.touchableRect(touch)
        outInsets.touchableInsets = InputMethodService.Insets.TOUCHABLE_INSETS_REGION
        outInsets.touchableRegion.set(touch)
    }

    /** 横屏打字的应用记下来，游戏键盘设置页按它列出可勾的应用。 */
    fun rememberLandscapeApp(packageName: String?) {
        if (packageName == null || packageName == context.packageName) return
        if (context.resources.configuration.orientation != Configuration.ORIENTATION_LANDSCAPE) return
        if (GameKeyboard.seen(context).any { it.packageName == packageName }) return
        val label = runCatching {
            val manager = context.packageManager
            manager.getApplicationLabel(manager.getApplicationInfo(packageName, 0)).toString()
        }.getOrDefault(packageName)
        host.writeSettings { GameKeyboard.remember(context, GameKeyboard.SeenApp(packageName, label)) }
    }

    /** 给当前应用开或关游戏键盘；开的时候总开关关着就一起打开。横屏马上换，竖屏提示横过来才换。 */
    fun toggle() {
        val app = host.editorPackage ?: return
        val on = enabledHere != true
        host.writeSettings {
            GameKeyboard.setApp(context, app, on)
            if (on) Settings.preferences(context).edit().putBoolean(GameKeyboard.KEY_ENABLED, true).commit()
        }
        host.rebuild()
        if (on && context.resources.configuration.orientation != Configuration.ORIENTATION_LANDSCAPE) {
            host.toast(context.getString(R.string.game_on_in_landscape))
        }
    }

    override fun onGameMoved(x: Float, y: Float) {
        val game = packageName ?: return
        host.writeSettings { GameKeyboard.savePosition(context, game, x, y) }
    }

    override fun onGameTransparency() {
        val steps = listOf(0, 20, 40, 60)
        val next = steps.firstOrNull { it > GameKeyboard.load(context).transparency } ?: steps.first()
        host.writeSettings {
            Settings.preferences(context).edit().putInt(GameKeyboard.KEY_TRANSPARENCY, next).commit()
        }
        frame?.setTransparency(next)
    }

    /** 拖动栏的键盘按钮：这个应用不再用游戏键盘，马上换回普通键盘（横屏照系统进全屏编辑）。 */
    override fun onGameNormalKeyboard() {
        val game = packageName ?: return
        host.writeSettings { GameKeyboard.setApp(context, game, on = false) }
        host.rebuild()
        host.toast(context.getString(R.string.game_switched_off))
    }

    override fun onGameHide() = host.hide()
}
