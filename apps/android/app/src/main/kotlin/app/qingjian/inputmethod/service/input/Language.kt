package app.qingjian.inputmethod.service.input

/**
 * 键盘能切的输入语言。中英键按一下在一对语言之间切，长按从这里挑另一种（见 [Composer.chooseLanguage]）；
 * [key] 写在中英键上，[title] 写在长按气泡里。加新语言在这里加一项，再在 [Composer] 里接上它的输入方式。
 */
enum class Language(val key: String, val title: String) {
    CHINESE("中", "中文"),
    ENGLISH("英", "English"),
    JAPANESE("日", "日本語"),
    ;

    companion object {
        fun of(name: String?, fallback: Language): Language = entries.firstOrNull { it.name == name } ?: fallback
    }
}
