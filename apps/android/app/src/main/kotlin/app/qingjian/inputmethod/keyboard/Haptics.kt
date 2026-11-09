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
 * 震不震只看键盘设置的「按键震动」，轻重看「震动强度」（[HapticStrength]）。
 */
object Haptics {
    private const val TAP_MS = 12L
    private const val HOLD_MS = 25L

    /** 「轻」在不支持合成振动的机器上退回的短震：时长与振幅（1–255）。 */
    private const val LIGHT_MS = 8L
    private const val LIGHT_AMPLITUDE = 60

    /** 「轻」用合成振动的 tick 原语，按这个比例缩小。 */
    private const val LIGHT_SCALE = 0.5f

    fun tap(context: Context) = vibrate(context, hold = false)

    /** 长按弹出气泡、进挪光标这类，比敲键重一点。 */
    fun hold(context: Context) = vibrate(context, hold = true)

    private fun vibrate(context: Context, hold: Boolean) {
        val prefs = Settings.preferences(context)
        if (!prefs.getBoolean(Settings.KEY_HAPTIC, true)) return
        val vibrator = vibrator(context)?.takeIf { it.hasVibrator() } ?: return
        val strength = HapticStrength.of(prefs.getString(Settings.KEY_HAPTIC_STRENGTH, null))
        val pattern = effect(vibrator, strength, hold)
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

    /**
     * 预置效果的强弱由厂商调好、不能缩放：标准是 TICK / CLICK（敲键 / 长按），重一档是 CLICK / HEAVY_CLICK；
     * 轻一档要更弱，只能用合成振动按比例缩小 tick，机器不支持时退回一个小振幅的短震。
     */
    private fun effect(vibrator: Vibrator, strength: HapticStrength, hold: Boolean): VibrationEffect {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.Q) {
            return when (strength) {
                HapticStrength.LIGHT -> VibrationEffect.createOneShot(LIGHT_MS, light(vibrator))
                HapticStrength.STANDARD -> VibrationEffect.createOneShot(if (hold) HOLD_MS else TAP_MS, VibrationEffect.DEFAULT_AMPLITUDE)
                HapticStrength.STRONG -> VibrationEffect.createOneShot((if (hold) HOLD_MS else TAP_MS) * 2, VibrationEffect.DEFAULT_AMPLITUDE)
            }
        }
        return when (strength) {
            HapticStrength.LIGHT -> lightEffect(vibrator, hold)
            HapticStrength.STANDARD -> VibrationEffect.createPredefined(if (hold) VibrationEffect.EFFECT_CLICK else VibrationEffect.EFFECT_TICK)
            HapticStrength.STRONG -> VibrationEffect.createPredefined(if (hold) VibrationEffect.EFFECT_HEAVY_CLICK else VibrationEffect.EFFECT_CLICK)
        }
    }

    private fun lightEffect(vibrator: Vibrator, hold: Boolean): VibrationEffect {
        val primitive = if (hold) VibrationEffect.Composition.PRIMITIVE_CLICK else VibrationEffect.Composition.PRIMITIVE_TICK
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R && vibrator.areAllPrimitivesSupported(primitive)) {
            return VibrationEffect.startComposition().addPrimitive(primitive, LIGHT_SCALE).compose()
        }
        return VibrationEffect.createOneShot(LIGHT_MS, light(vibrator))
    }

    private fun light(vibrator: Vibrator) = if (vibrator.hasAmplitudeControl()) LIGHT_AMPLITUDE else VibrationEffect.DEFAULT_AMPLITUDE

    private fun vibrator(context: Context): Vibrator? = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
        context.getSystemService(VibratorManager::class.java)?.defaultVibrator
    } else {
        @Suppress("DEPRECATION")
        context.getSystemService(Vibrator::class.java)
    }
}
