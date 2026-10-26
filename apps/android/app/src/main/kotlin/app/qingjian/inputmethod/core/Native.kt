package app.qingjian.inputmethod.core

/** Rust 侧（apps/android/core）的 JNI 入口。句柄只在主线程用。 */
internal object Native {
    init {
        System.loadLibrary("qingjian_android")
    }

    /** 失败返回 0，原因在 logcat（标签 qingjian）。 */
    external fun create(dataDir: String, userDir: String): Long

    /** 「青简翻译」用的轻量句柄（只能 setCloud / translate / pollTranslation / destroy）；[language] 是学习语言代码。失败返回 0。 */
    external fun createTranslation(dataDir: String, userDir: String, language: String): Long

    external fun destroy(handle: Long)

    external fun push(handle: Long, codePoint: Int): String?

    external fun backspace(handle: Long): String?

    /** 滑完一笔（`{points, keys}` 的 JSON，像素）：解出的字母进组句，回一帧。 */
    external fun swipe(handle: Long, input: String): String?

    /** 只解码不进组句，回字母串（解不出是空串）。 */
    external fun decodeSwipe(handle: Long, input: String): String?

    external fun select(handle: Long, index: Int): String?

    external fun selectTranslation(handle: Long, index: Int, sense: Int): String?

    external fun forget(handle: Long, index: Int): String?

    external fun commitRaw(handle: Long): String?

    external fun clear(handle: Long): String?

    external fun setNineKey(handle: Long, on: Boolean): String?

    external fun pin(handle: Long, alternative: String): String?

    external fun setEnglishMode(handle: Long, on: Boolean): String?

    external fun setZhuyin(handle: Long, on: Boolean): String?

    /** 双拼方案（空串为全拼）与 preedit 是否显示敲的键。 */
    external fun setShuangpin(handle: Long, scheme: String, rawPreedit: Boolean): String?

    /** 五笔开关；[mixed] 时拼音也开着（混输）。 */
    external fun setWubi(handle: Long, on: Boolean, mixed: Boolean): String?

    external fun setOptions(handle: Long, options: String): String?

    external fun punctuate(handle: Long, c: Char): String?

    /** 日语罗马音模式开关：开着时敲键、选词走 Core 的罗马音缓冲区。 */
    external fun setJapanese(handle: Long, on: Boolean): String?

    /** 繁体输出开关；字形按键盘选项里的。 */
    external fun setTraditional(handle: Long, on: Boolean): String?

    /** 组句中按空格：注音缺声调时是一声，否则上屏第一个候选。 */
    external fun space(handle: Long): String?

    /** 不在组句时按光标前的文字给补全（邮箱后缀），放在帧的候选里。 */
    external fun complete(handle: Long, beforeCursor: String): String?

    external fun reset(handle: Long)

    /** 画最新一帧的候选条，返回尺寸与可点区域的 JSON；[highlighted] 为负不高亮，[visibleWidth] 以内的候选记成看到过（生词本次数）。画不出来返回 null。 */
    external fun renderStrip(handle: Long, highlighted: Int, scale: Float, dark: Boolean, visibleWidth: Int): String?

    /** 候选条换主题（内置主题 id）与字号档（[textScale] 是候选字与译文一起的倍数）。 */
    external fun setAppearance(handle: Long, theme: String, textScale: Float)

    /** 内置主题列表的 JSON（`[{id, name}]`），按 PC 设置程序的顺序。 */
    external fun themes(): String?

    /** 主题在深 / 浅色下的键盘配色（JSON，颜色是 ARGB）；没有这个主题时给缺省主题的。 */
    external fun keyboardPalette(theme: String, dark: Boolean): String?

    /** 主题预览：前 8 字节是宽、高（大端），后面是预乘 RGBA；画不出为 null。渲染器不能跨线程，固定在一条线程上调。 */
    external fun themePreview(theme: String, dark: Boolean, scale: Float, textScale: Float): ByteArray?

    /** 把上次 [renderStrip] 的像素（预乘 RGBA）拷进直接缓冲区；缓冲区不够大返回 false。 */
    external fun copyStrip(handle: Long, buffer: java.nio.ByteBuffer): Boolean

    /** 符号 / 表情面板目录的 JSON（[{group, name, items}]），不要句柄；出错返回 null。 */
    external fun panelCatalog(): String?

    /** 剪贴板操作（[request] 是 `{"op":…}` 的 JSON），回操作之后的列表 JSON（[{text, image}]）；[limit] 是最大条数。 */
    external fun clipboard(userDir: String, request: String, limit: Int): String?

    /** 记一张复制的图片，回操作之后的列表 JSON。 */
    external fun clipboardAddImage(userDir: String, bytes: ByteArray, extension: String, limit: Int): String?

    /** 常用语操作（[request] 是 `{"op":…}` 的 JSON），回操作之后的整页 JSON（{groups, phrases}）。 */
    external fun phrases(userDir: String, request: String): String?

    /** 常用语编辑的规则（{default_group, max_code}），不要句柄。 */
    external fun phraseRules(): String?

    /** 剪贴板卡片的代码显示 JSON（{code, spans: [[起, 止, 种类]]}，UTF-16 偏移），不要句柄。 */
    external fun codeView(text: String): String?

    /** 拆词：一段文字切成可点选的小块（JSON 字符串数组）。 */
    external fun textPieces(handle: Long, text: String): String?

    /** 常用语改过之后重挂带输入码的那些。 */
    external fun reloadPhrases(handle: Long, userDir: String)

    /** 接上（[settings] 是含密钥的配置 JSON）或断开（null）云端，返回接没接上。 */
    external fun setCloud(handle: Long, settings: String?): Boolean

    /** 开（[settings] 是含密钥的配置 JSON）或关（null）释义兜底，返回开没开上。 */
    external fun setGlossFill(handle: Long, settings: String?): Boolean

    /** 这段组句的光标前后文（云联想用），组句开头交一次。 */
    external fun setSurrounding(handle: Long, before: String, after: String)

    /** 云联想结果插进候选后的一帧 JSON；没有新结果为 null。 */
    external fun pollPrediction(handle: Long): String?

    /** 快捷翻译发一条；没接云端或文字为空返回 false。 */
    external fun translate(handle: Long, text: String): Boolean

    /** 译文：null 是还没到，空串是模型没给出内容。 */
    external fun pollTranslation(handle: Long): String?

    external fun cancelTranslation(handle: Long)

    /** 上屏记不记进输入统计；工具框（快捷翻译、常用语编辑）开着时关。 */
    external fun setMetered(handle: Long, metered: Boolean)

    /** 私密输入：应用声明不要学习的输入框里不学习、不记统计与生词、不发云端。 */
    external fun setPrivate(handle: Long, private: Boolean)

    /** 测试连接，阻塞到有结果（请求自带超时），只能在后台线程调；回 `{ok, message}`。 */
    external fun cloudTest(settings: String): String?

    /** 键盘设置页的选项清单与缺省值（{fuzzy, learning_languages, traditional_variants, …}），不要句柄。 */
    external fun settingChoices(): String?

    /** 算式里按了 [c]：回要交给 [push] 的字符（× ÷ 换成 * /），不是算式字符回 0。 */
    external fun expressionChar(c: Char): Char

    /** 光标前最后一个字是 [c] 时可能是一道算式的结尾，才去问补全。 */
    external fun mayEndCalculation(c: Char): Boolean

    /** 双拼方案列表的 JSON（[{key, label}]），不要句柄。 */
    external fun shuangpinChoices(): String?

    /** 一个双拼方案每个键的声韵母（[{key, initial, finals}]），不要句柄。 */
    external fun shuangpinKeycaps(scheme: String): String?

    /** 一个 emoji 的五档肤色（JSON 数组）；不能接肤色是空数组，出错返回 null。 */
    external fun skinTones(text: String): String?

    /** 「符号」或「表情」组的「常用」该显示的条目（JSON 数组），用过的在前；出错返回 null。 */
    external fun panelCommon(userDir: String, group: String): String?

    /** 面板里点了一格，记一次使用（[userDir] 里的 panel-usage.tsv）。 */
    external fun recordPanelUse(userDir: String, item: String)

    /** 输入统计（今天、近 7 天、累计）的 JSON，读 [userDir] 里的统计文件；出错返回 null。 */
    external fun usageStats(userDir: String): String?

    /** 统计详情页的 JSON（[language] 是学习语言的代码，等级表在 [dataDir]）；出错返回 null。 */
    external fun statsDetail(dataDir: String, userDir: String, language: String): String?

    /** 生词本：按 [request]（`{language, filter, level, change}` 的 JSON）先改再列，回汇总与筛出的词条；出错返回 null。 */
    external fun vocabulary(dataDir: String, userDir: String, request: String): String?

    /** 导出数据要打包的文件（JSON：[{name, path}]）；出错返回 null。 */
    external fun learningFiles(userDir: String): String?

    /** 删掉输入习惯（[habits]）与生词本（[vocabulary]）的文件，回删掉几个，出错 -1。调之前要先卸下开着的引擎。 */
    external fun clearLearning(userDir: String, habits: Boolean, vocabulary: Boolean): Int

    /** 词库页：按 [request]（`{import}` 或 `{remove}` 的 JSON，都不带只列）先导入或删除再列，回领域词库、导入的词库与一句结果；出错返回 null。 */
    external fun dictionaries(dataDir: String, userDir: String, request: String): String?

    /** 识别出的一段文字上屏（Core 转繁体、记历史与统计），回一帧。 */
    external fun commitVoice(handle: Long, text: String): String?

    /** 按「语音识别」设置换识别方（[settings] 为 null 是没配）。 */
    external fun setVoice(handle: Long, settings: String?)

    /** 「语音识别」页的测试连接，阻塞到有结果，只能在后台线程调；回 `{ok, message}`。 */
    external fun voiceTest(settings: String): String?

    /** Rust 侧的识别方（云端、离线模型）能不能用。 */
    external fun voiceAvailable(handle: Long): Boolean

    /** Rust 侧识别开始一段；之后交的录音是 [sampleRate] 的单声道 16 位 PCM。 */
    external fun voiceStart(handle: Long, sampleRate: Int)

    external fun voiceFeed(handle: Long, samples: ShortArray)

    /** 说完了，等最终结果。 */
    external fun voiceFinish(handle: Long)

    external fun voiceCancel(handle: Long)

    /** 识别的一件事（`{kind, text}`，kind 是 partial / final / error）；没有为 null。 */
    external fun pollVoice(handle: Long): String?
}
