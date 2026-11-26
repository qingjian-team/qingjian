//! 适配 Android 物理按键的消费配对、修饰键与设备生命周期。

package app.qingjian.inputmethod.service.input.physical

import app.qingjian.inputmethod.service.input.Composer
import android.content.Context
import android.hardware.input.InputManager
import android.view.KeyCharacterMap
import android.view.KeyEvent
import app.qingjian.inputmethod.keyboard.layout.Key
import app.qingjian.inputmethod.settings.PhysicalShortcuts

/** Android 事件的入口：匹配抬起、区分单击修饰键与组合键，输入和选词交给 Composer/Rust。 */
class Keyboard(
    private val context: Context,

    private val composer: Composer,

    private val changed: () -> Unit,
) : InputManager.InputDeviceListener {
    private val manager = context.getSystemService(InputManager::class.java)

    private val tap = ModifierTap()

    private val presses = Presses()

    private val consumed = mutableSetOf<Pair<Int, Int>>()

    private val pressed = mutableSetOf<Pair<Int, Int>>()

    private var shortcuts = PhysicalShortcuts.load(context)

    var active = false
        private set

    fun start() { manager?.registerInputDeviceListener(this, null) }

    fun stop() { manager?.unregisterInputDeviceListener(this) }

    fun reload() { shortcuts = PhysicalShortcuts.load(context); tap.cancel() }

    /** 焦点变化后旧按键不再影响新输入框。 */
    fun reset() {
        tap.cancel()
        consumed.clear()
        presses.clear()
        pressed.clear()
        active = false
        composer.physicalInput = false
    }

    fun down(event: KeyEvent): Boolean {
        if (!hardware(event)) return false
        if (event.isCanceled) { tap.cancel(); return false }
        val key = event.deviceId to event.keyCode
        val initial = event.repeatCount == 0
        pressed += key
        presses.down(event.deviceId, event.keyCode, event.modifiers, initial)
        val plain = !event.isCtrlPressed && !event.isAltPressed && !event.isMetaPressed
        val switchModifier = when (event.keyCode) {
            KeyEvent.KEYCODE_SHIFT_LEFT, KeyEvent.KEYCODE_SHIFT_RIGHT -> shortcuts.shift
            KeyEvent.KEYCODE_CTRL_LEFT, KeyEvent.KEYCODE_CTRL_RIGHT -> shortcuts.control
            else -> false
        }
        val bothShifts = event.metaState and KeyEvent.META_SHIFT_LEFT_ON != 0 && event.metaState and KeyEvent.META_SHIFT_RIGHT_ON != 0
        val bothControls = event.metaState and KeyEvent.META_CTRL_LEFT_ON != 0 && event.metaState and KeyEvent.META_CTRL_RIGHT_ON != 0
        if (switchModifier && initial && pressed.size == 1 && !bothShifts && !bothControls && !event.isAltPressed && !event.isMetaPressed &&
            !(event.isShiftPressed && event.isCtrlPressed) && !composer.directInput
        ) {
            tap.begin(event.deviceId, event.keyCode, event.downTime)
            // 修饰键按下透传，使应用的组合键仍收到完整的 down/up。
            return false
        }
        tap.cancel()
        if (event.keyCode == KeyEvent.KEYCODE_SPACE && shortcuts.ctrlAltSpace &&
            event.isCtrlPressed && event.isAltPressed && !event.isShiftPressed && !event.isMetaPressed && !composer.directInput
        ) {
            activate()
            if (initial) composer.onKey(Key.Language)
            consumed += key
            return true
        }
        if (!initial && key in consumed && presses.changed(event.deviceId, event.keyCode, event.modifiers)) return true
        if (!plain || composer.directInput) {
            if (!KeyEvent.isModifierKey(event.keyCode)) composer.preparePhysicalPassthrough()
            return false
        }
        if (KeyEvent.isModifierKey(event.keyCode)) return false
        activate()
        val action = when (event.keyCode) {
            KeyEvent.KEYCODE_SPACE -> if (shortcuts.enterCandidate) Action.RAW else Action.CANDIDATE
            KeyEvent.KEYCODE_ENTER, KeyEvent.KEYCODE_NUMPAD_ENTER -> if (shortcuts.enterCandidate) Action.CANDIDATE else Action.RAW
            KeyEvent.KEYCODE_DEL -> Action.BACKSPACE
            KeyEvent.KEYCODE_FORWARD_DEL -> Action.DELETE
            KeyEvent.KEYCODE_ESCAPE -> Action.CLEAR
            KeyEvent.KEYCODE_DPAD_LEFT -> Action.LEFT
            KeyEvent.KEYCODE_DPAD_RIGHT -> Action.RIGHT
            KeyEvent.KEYCODE_MOVE_HOME -> Action.HOME
            KeyEvent.KEYCODE_MOVE_END -> Action.END
            KeyEvent.KEYCODE_PAGE_UP -> Action.PREVIOUS_PAGE
            KeyEvent.KEYCODE_PAGE_DOWN -> Action.NEXT_PAGE
            KeyEvent.KEYCODE_DPAD_UP -> Action.PREVIOUS
            KeyEvent.KEYCODE_DPAD_DOWN -> Action.NEXT
            else -> Action.CHARACTER
        }
        val unicode = event.unicodeChar
        val character = unicode.takeIf { it > 0 && it and KeyCharacterMap.COMBINING_ACCENT == 0 }
        var handled = !event.isShiftPressed && action != Action.CHARACTER &&
            composer.onPhysical(action, 0, !initial)
        if (action == Action.CHARACTER && character != null) {
            handled = composer.onPhysical(action, character, !initial)
        }
        if (!handled && character != null && action == Action.CHARACTER && character in 33..126) {
            // 标点沿用屏幕键盘的 Core 标点与成对符号调用，不在壳里转换。
            handled = composer.onPhysicalPunctuation(character.toChar())
        }
        if (handled || key in consumed) {
            consumed += key
            changed()
            return true
        }
        if (composer.hasComposition && (action != Action.CHARACTER || character == null || !handled)) composer.preparePhysicalPassthrough()
        changed()
        return false
    }

    fun up(event: KeyEvent): Boolean {
        if (!hardware(event)) return false
        val key = event.deviceId to event.keyCode
        pressed.remove(key)
        presses.up(event.deviceId, event.keyCode)
        val switch = tap.release(event.deviceId, event.keyCode, event.downTime,
            event.isCanceled || event.eventTime - event.downTime > android.view.ViewConfiguration.getLongPressTimeout())
        // 抬起事件的 `isShiftPressed`/`isCtrlPressed` 可能仍包含正在抬起的键。
        // 单击判定已经排除了组合键，这里不能再检查该修饰键自身的状态。
        if (switch && !event.isAltPressed && !event.isMetaPressed && !composer.directInput) {
            activate()
            composer.onKey(Key.Language)
            changed()
        }
        // 单击修饰键的 down 没有消费，up 同样透传；只有真正消费的按键成对拿走。
        return consumed.remove(key)
    }

    fun touch() {
        if (!active) return
        active = false
        composer.restoreTouchInput()
        tap.cancel()
        consumed.clear()
        presses.clear()
        pressed.clear()
        changed()
    }

    private fun activate() {
        if (active) return
        active = true
        composer.physicalInput = true
    }

    private fun hardware(event: KeyEvent): Boolean =
        event.flags and KeyEvent.FLAG_SOFT_KEYBOARD == 0 && event.deviceId != KeyCharacterMap.VIRTUAL_KEYBOARD &&
            event.isFromSource(android.view.InputDevice.SOURCE_KEYBOARD)

    private fun deviceChanged() {
        tap.cancel()
        consumed.clear()
        presses.clear()
        pressed.clear()
        changed()
    }

    override fun onInputDeviceAdded(deviceId: Int) = deviceChanged()
    override fun onInputDeviceChanged(deviceId: Int) = deviceChanged()
    override fun onInputDeviceRemoved(deviceId: Int) {
        if (active) {
            active = false
            composer.restoreTouchInput()
        }
        deviceChanged()
    }
}
