package app.qingjian.inputmethod.core.store

/** 剪贴板历史的一条：文字原样，或图片文件的完整路径（[image] 非空）。 */
class ClipItem(val text: String, val image: String?)
