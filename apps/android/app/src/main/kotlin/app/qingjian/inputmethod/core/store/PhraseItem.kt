package app.qingjian.inputmethod.core.store

/** 一条常用语；[index] 是它在全部常用语里的位置（改、删、移到最前都按它）。 */
class PhraseItem(val index: Int, val text: String, val group: String, val code: String)
