package app.qingjian.inputmethod.candidates

import app.qingjian.inputmethod.core.strip.StripImage

/** 候选条的位图从哪来：服务转给 Rust 渲染器画最新一帧；[highlighted] 为负不高亮，[visibleWidth] 是视口宽（像素）。 */
fun interface StripSource {
    fun render(highlighted: Int, dark: Boolean, visibleWidth: Int): StripImage?
}
