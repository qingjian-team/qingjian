package app.qingjian.inputmethod.keyboard

import android.content.Context
import android.os.Build
import android.os.VibrationAttributes
import android.os.VibrationEffect
import android.os.Vibrator
import android.os.VibratorManager
import app.qingjian.inputmethod.settings.Settings

/**
 * 按键震动。直接调振动器（要 `VIBRATE` 权限），不用 `View.performHapticFeedback`：
 * 后者跟着系统的「触感反馈」走，系统那边关着（不少机型缺省关，振动会被记成 ignored_for_settings）设置里开了也不震。
 * 震不震只看键盘设置的「按键震动」。
 */
object Haptics {
    private const val TAP_MS = 12L
    private const val HOLD_MS = 25L

    fun tap(context: Context) = vibrate(context, TAP_MS, Effect.TICK)

    /** 长按弹出气泡、进挪光标这类，比敲键重一点。 */
    fun hold(context: Context) = vibrate(context, HOLD_MS, Effect.CLICK)

    private enum class Effect { TICK, CLICK }

    private fun vibrate(context: Context, millis: Long, effect: Effect) {
        if (!Settings.preferences(context).getBoolean(Settings.KEY_HAPTIC, true)) return
        val vibrator = vibrator(context)?.takeIf { it.hasVibrator() } ?: return
        val pattern = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            VibrationEffect.createPredefined(
                if (effect == Effect.TICK) VibrationEffect.EFFECT_TICK else VibrationEffect.EFFECT_CLICK,
            )
        } else {
            VibrationEffect.createOneShot(millis, VibrationEffect.DEFAULT_AMPLITUDE)
        }
        runCatching {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                // 不带用途的短震会被归成 TOUCH，跟着系统「触摸振动」走（不少机型缺省关，直接丢掉）；
                // PHYSICAL_EMULATION 正是给键盘模拟实体按键手感的，归「硬件反馈」强度管
                vibrator.vibrate(pattern, VibrationAttributes.createForUsage(VibrationAttributes.USAGE_PHYSICAL_EMULATION))
            } else {
                vibrator.vibrate(pattern)
            }
        }
    }

    private fun vibrator(context: Context): Vibrator? = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
        context.getSystemService(VibratorManager::class.java)?.defaultVibrator
    } else {
        @Suppress("DEPRECATION")
        context.getSystemService(Vibrator::class.java)
    }
}
