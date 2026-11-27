//! 根据当前输入语言与方案刷新屏幕键盘键位。

package app.qingjian.inputmethod.service

import app.qingjian.inputmethod.core.store.Shuangpin
import app.qingjian.inputmethod.keyboard.layout.KeyboardLayout
import app.qingjian.inputmethod.service.input.Composer
import app.qingjian.inputmethod.service.input.Language

internal fun InputPanel.presentKeyboard(composer: Composer, lettersOnly: Boolean, enterLabel: String?) {
    keyboard.chinese = composer.chinese
    keyboard.languageLabel = composer.language.key
    keyboard.languageHint = composer.partner.key
    keyboard.languageChoices = Language.entries.map { it.title }
    keyboard.languageChoice = composer.language.ordinal
    keyboard.enterLabel = enterLabel
    val fullWidth = composer.options.fullWidthPunctuation
    // 滑行只给拼音全键盘与英文键盘（常用语编辑条的英文键盘也算）：双拼、五笔、日语的字母不是这样拼词的
    keyboard.swipeLayout = composer.page == Composer.Page.LETTERS && (
        lettersOnly || (!composer.nineKey && !composer.zhuyin && !composer.shuangpin && !composer.wubi && !composer.japanese)
    )
    when {
        composer.page == Composer.Page.NUMBERS -> keyboard.setRows(KeyboardLayout.numbers(), side = true)
        composer.page == Composer.Page.SYMBOLS -> keyboard.setRows(KeyboardLayout.symbols(composer.chinese, fullWidth))
        // 常用语的输入码只收字母，九宫格、注音打不出来，换成英文全键盘
        lettersOnly -> keyboard.setRows(KeyboardLayout.qwerty(chinese = false))
        composer.nineKey -> keyboard.setRows(KeyboardLayout.nineKey(), side = true)
        composer.zhuyin -> keyboard.setRows(KeyboardLayout.zhuyin(fullWidth))
        composer.shuangpin -> keyboard.setRows(
            KeyboardLayout.shuangpin(
                Shuangpin.keycaps(composer.shuangpinScheme), composer.shuangpinScheme, fullWidth,
                hints = settings.shuangpinHints,
            ),
        )
        else -> keyboard.setRows(KeyboardLayout.qwerty(composer.chinese, composer.japanese, fullWidth))
    }
}

/** 左列显示九宫格拼音或当前页标点。 */
internal fun InputPanel.presentSideItems(composer: Composer, pinyinChoices: List<String>) {
    keyboard.setSideItems(when {
        composer.page == Composer.Page.NUMBERS -> KeyboardLayout.NUMBER_SIDE
        else -> pinyinChoices.ifEmpty { KeyboardLayout.sidePunctuation(composer.options.fullWidthPunctuation) }
    })
}
