package app.qingjian.inputmethod.panel

import app.qingjian.inputmethod.core.store.PhraseItem
import app.qingjian.inputmethod.settings.Settings

/** 面板里能做的事，由输入法服务实现。 */
interface PanelActions {
    fun openPanel(kind: PanelKind)

    /** 回到键盘。 */
    fun closePanel()

    /** 面板里点了一个符号或表情：组句中先顶首选再上屏。 */
    fun commitText(text: String)

    fun deleteBackward()

    /** 候选展开里点了第 [index] 个候选。 */
    fun selectCandidate(index: Int)

    /** 清空组句（候选展开里的「重输」）。 */
    fun clearComposition()

    /** 回到键盘并进入拖动调高度。 */
    fun startResize()

    fun openSettings()

    fun switchInputMethod()

    /** 选了拼音键盘：回到中文并换成这个布局。 */
    fun chooseLayout(layout: Settings.Layout)

    /** 选了英文 26 键：切到英文（全键盘），拼音布局不动，中英键切回来还是它。 */
    fun chooseEnglish()

    /** 选了日语罗马音（测试版：假名 + 小词表汉字）。 */
    fun chooseJapanese()

    /** 现在是不是英文，键盘选择里给它打勾。 */
    val english: Boolean

    val japanese: Boolean

    /** 就地改一个开关设置（[Settings] 里的键）。 */
    fun setToggle(key: String, on: Boolean)

    /** 剪贴板里点了一张图（图片文件路径）：发进输入框，输入框不收图片就放回系统剪贴板。 */
    fun commitImage(path: String)

    /** 剪贴板里点了「拆词」。 */
    fun openPieces(text: String)

    /** 拆词：按引擎的语言模型切成小块。 */
    fun textPieces(text: String): List<String>

    /** 编辑常用语：[phrase] 为 null 是新加一条，放进 [group]。 */
    fun editPhrase(phrase: PhraseItem?, group: String)

    /** 常用语改过：带输入码的重新挂进候选。 */
    fun phrasesChanged()

    /** 拆词选着的字放进输入框，处在编辑中（组合文字，带下划线）；空串收掉。 */
    fun composeText(text: String)

    /** 拆词确认：编辑中的字定下来上屏。 */
    fun finishComposeText()

    /** 放进系统剪贴板（拆词的「复制」）。 */
    fun copyToClipboard(text: String)

    fun toast(message: String)

    /** 剪贴板最多存多少条（设置里的）。 */
    val clipboardLimit: Int
}
