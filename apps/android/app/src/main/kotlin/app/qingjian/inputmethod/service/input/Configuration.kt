//! 输入设置与外观在无软键盘视图时同样生效。

package app.qingjian.inputmethod.service.input

import app.qingjian.inputmethod.core.InputOptions
import app.qingjian.inputmethod.core.Engine
import app.qingjian.inputmethod.settings.Settings

/** 输入设置在 onStartInput 生效，不能等屏幕键盘创建后才传给引擎。 */
fun Composer.configure(settings: Settings, glosses: Boolean = true) {
    shuangpinScheme = settings.shuangpinScheme
    shuangpinRawPreedit = settings.shuangpinRawPreedit
    wubiMixed = settings.wubiMixed
    layout = settings.layout
    traditional = settings.traditional
    keepChinese = settings.keepChinese
    options = settings.inputOptions.let { if (glosses) it else it.copy(learningLanguage = InputOptions.LEARNING_OFF) }
    englishCandidates = settings.englishCandidates
    applyScheme()
}

/** 无视图场景也要应用主题与字号。 */
fun Engine.configureAppearance(settings: Settings) = setAppearance(settings.theme, settings.textSize.scale)
