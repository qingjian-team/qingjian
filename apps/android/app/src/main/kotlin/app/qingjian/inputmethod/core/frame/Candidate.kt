package app.qingjian.inputmethod.core.frame

data class Candidate(
    val text: String,
    /** 学习语言下的译文，最多两条；下标就是上屏译词时的 sense。 */
    val senses: List<Sense>,
    /** 长按能删（用户词、学习记录、个人英文词）；整句、emoji 这些没什么可删。 */
    val forgettable: Boolean = false,
    /** 长按能屏蔽：只对词库来的词给（用户词与自定义短语是用户自己写的，屏蔽词管不到）。 */
    val blockable: Boolean = false,
    /** 来自云联想（云端词、整句补全），画成云端色。 */
    val cloud: Boolean = false,
    /** 通讯录里的名字，候选条在词的右上角画一个人头。 */
    val contact: Boolean = false,
)
