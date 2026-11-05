package app.qingjian.inputmethod.settings.home

import android.app.Activity
import android.app.AlertDialog
import android.os.Handler
import android.os.Looper
import android.text.Editable
import android.text.TextUtils
import android.text.TextWatcher
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputMethodManager
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.EditText
import android.widget.HorizontalScrollView
import android.widget.LinearLayout
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.core.store.SettingChoices
import app.qingjian.inputmethod.core.store.Vocabulary
import app.qingjian.inputmethod.core.store.Vocabulary.Filter
import app.qingjian.inputmethod.core.store.VocabularyWord
import app.qingjian.inputmethod.settings.Settings
import app.qingjian.inputmethod.settings.widget.SettingsWidgets

/**
 * 生词本 tab：打字时候选旁见过的学习语言译词。固定的顶栏里是标题、搜索框与 全部 / 生词 / 近 7 天 / 认识了 分段，
 * 下面是等级筛选、词数与学习语言、词表（最近见到的在前）；点一个词弹出详情，可标认识或删除（同 PC 生词本的「词表」，时间线不做）。
 * 改了只重填这一列，滚动位置不丢。
 */
class VocabularyPage(private val activity: Activity, private val ui: SettingsWidgets) : AppPage {
    private val colors get() = ui.colors

    private var query = Vocabulary.Query()

    /** 画着的是哪门语言、多少词；回到前台时变了才重建。 */
    private var shown: Triple<String, Long, Long>? = null

    private var column: LinearLayout? = null

    /** 搜索框下面的数目与词表；搜索时只重填这一块，搜索框不重建、不丢焦点。 */
    private var results: LinearLayout? = null

    private val handler = Handler(Looper.getMainLooper())

    private val searchNow = Runnable { reloadResults() }

    private val search: EditText by lazy { searchField() }

    /** 顶栏里搜索框与分段那一块；没开学习语言时藏起来。 */
    private var tools: LinearLayout? = null

    private fun signature(vocabulary: Vocabulary) = Triple(vocabulary.language, vocabulary.seen, vocabulary.matched)

    override fun stale() = signature(load()) != shown

    /** 换了学习语言后原来选的等级可能没有了，退回全部等级。 */
    private fun load(): Vocabulary {
        val vocabulary = Vocabulary.load(activity, query)
        if (query.level.isEmpty() || query.level in vocabulary.levels) return vocabulary
        query = query.copy(level = "")
        return Vocabulary.load(activity, query)
    }

    override fun header(): View = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        addView(ui.pageTitle(R.string.tab_vocabulary))
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(ui.dp(12), 0, ui.dp(12), ui.dp(10))
            tools = this
        })
    }

    /** 搜索框与筛选分段；换了筛选只重建这一块，搜索框原样挪回来、不丢焦点。 */
    private fun fillTools(vocabulary: Vocabulary) {
        val tools = tools ?: return
        tools.removeAllViews()
        tools.visibility = if (vocabulary.off) View.GONE else View.VISIBLE
        if (vocabulary.off) return
        (search.parent as? ViewGroup)?.removeView(search)
        tools.addView(search, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))
        tools.addView(ui.segmented(Filter.entries.map(::filterLabel), query.filter.ordinal) { index ->
            reload(query.copy(filter = Filter.entries[index]))
        }, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT).apply { topMargin = ui.dp(10) })
    }

    override fun build(): View = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        column = this
        fill(load())
    }

    private fun refill(vocabulary: Vocabulary) {
        column?.run {
            removeAllViews()
            fill(vocabulary)
        }
    }

    private fun LinearLayout.fill(vocabulary: Vocabulary) {
        shown = signature(vocabulary)
        fillTools(vocabulary)
        if (vocabulary.off) {
            addView(languageRow(vocabulary), spaced(16))
            addView(messageCard(activity.getString(R.string.vocabulary_off)))
            return
        }
        if (vocabulary.levels.isNotEmpty()) {
            val levels = listOf("") + vocabulary.levels
            addView(chips(levels.map { (it.ifEmpty { activity.getString(R.string.vocabulary_all_levels) }) to (it == query.level) }) { index ->
                reload(query.copy(level = levels[index]))
            }, spaced(14))
        }
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            results = this
            fillResults(vocabulary)
        })
    }

    private fun LinearLayout.fillResults(vocabulary: Vocabulary) {
        addView(languageRow(vocabulary), spaced(if (vocabulary.levels.isEmpty()) 16 else 12))
        when {
            vocabulary.words.isNotEmpty() -> addView(wordsCard(vocabulary.words))
            vocabulary.seen == 0L -> addView(messageCard(activity.getString(R.string.vocabulary_empty)))
            else -> addView(messageCard(activity.getString(R.string.vocabulary_no_match)))
        }
        if (vocabulary.matched > vocabulary.words.size) {
            addView(ui.text(activity.getString(R.string.vocabulary_truncated, vocabulary.words.size), 12f, colors.secondary).apply {
                gravity = Gravity.CENTER
                setPadding(0, ui.dp(12), 0, 0)
            })
        }
    }

    private fun reload(next: Vocabulary.Query) {
        query = next
        refill(load())
    }

    private fun reloadResults() {
        val vocabulary = load()
        shown = signature(vocabulary)
        results?.run {
            removeAllViews()
            fillResults(vocabulary)
        }
    }

    /** 搜索框：停手 [SEARCH_DELAY_MS] 后再查，每敲一个字都读一遍文件太勤。 */
    private fun searchField() = EditText(activity).apply {
        setHint(R.string.vocabulary_search_hint)
        setHintTextColor(colors.faint)
        setTextColor(colors.text)
        textSize = 15f
        isSingleLine = true
        imeOptions = EditorInfo.IME_ACTION_SEARCH
        background = ui.rounded(colors.fill, 10f)
        setPadding(ui.dp(14), ui.dp(9), ui.dp(14), ui.dp(9))
        // 回车键是「搜索」：结果已跟着打字出来，按下只收起键盘
        setOnEditorActionListener { view, _, _ ->
            activity.getSystemService(InputMethodManager::class.java)?.hideSoftInputFromWindow(view.windowToken, 0)
            true
        }
        addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) = Unit
            override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) = Unit
            override fun afterTextChanged(s: Editable?) {
                val text = s?.toString().orEmpty()
                if (text == query.search) return
                query = query.copy(search = text)
                handler.removeCallbacks(searchNow)
                handler.postDelayed(searchNow, SEARCH_DELAY_MS)
            }
        })
    }

    private fun spaced(topDp: Int) = LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT).apply { topMargin = ui.dp(topDp) }

    private fun filterLabel(filter: Filter) = activity.getString(
        when (filter) {
            Filter.ALL -> R.string.vocabulary_filter_all
            Filter.FRESH -> R.string.vocabulary_filter_fresh
            Filter.WEEK -> R.string.vocabulary_filter_week
            Filter.KNOWN -> R.string.vocabulary_filter_known
        },
    )

    /** 词表上方一行：左边词数（没开学习语言时是一句说明），右边学习语言，点开可换（与设置页里的同一项）。 */
    private fun languageRow(vocabulary: Vocabulary) = LinearLayout(activity).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        setPadding(ui.dp(16), 0, ui.dp(4), ui.dp(7))
        val left = if (vocabulary.off) activity.getString(R.string.vocabulary_intro) else activity.getString(R.string.vocabulary_count, vocabulary.matched)
        addView(ui.text(left, 13f, colors.secondary), LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
        addView(ui.text(SettingChoices.learningLanguageName(vocabulary.language) + " ⌄", 13f, colors.brand).apply {
            setPadding(ui.dp(12), ui.dp(4), ui.dp(12), ui.dp(4))
            setOnClickListener { pickLanguage(vocabulary.language) }
        })
    }

    private fun pickLanguage(current: String) {
        val languages = SettingChoices.learningLanguages
        AlertDialog.Builder(activity)
            .setTitle(R.string.learning_language_title)
            .setSingleChoiceItems(languages.map { it.label }.toTypedArray(), languages.indexOfFirst { it.key == current }) { dialog, which ->
                dialog.dismiss()
                Settings.preferences(activity).edit().putString(Settings.KEY_LEARNING_LANGUAGE, languages[which].key).apply()
                reload(Vocabulary.Query(search = query.search))
            }
            .show()
    }

    /** 一排可横滑的圆角选项，选中的填品牌色。 */
    private fun chips(items: List<Pair<String, Boolean>>, onPick: (Int) -> Unit) = HorizontalScrollView(activity).apply {
        isHorizontalScrollBarEnabled = false
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            items.forEachIndexed { index, (label, selected) ->
                addView(ui.text(label, 13.5f, if (selected) colors.brand else colors.text, bold = selected).apply {
                    background = ui.rounded(if (selected) colors.brandSoft else colors.surface, 16f)
                    setPadding(ui.dp(14), ui.dp(7), ui.dp(14), ui.dp(7))
                    setOnClickListener { if (!selected) onPick(index) }
                }, LinearLayout.LayoutParams(WRAP_CONTENT, WRAP_CONTENT).apply { if (index > 0) marginStart = ui.dp(8) })
            }
        })
    }

    private fun messageCard(message: String) = ui.card(
        ui.text(message, 13.5f, colors.secondary).apply {
            gravity = Gravity.CENTER
            setPadding(ui.dp(24), ui.dp(36), ui.dp(24), ui.dp(36))
            setLineSpacing(0f, 1.3f)
        },
    )

    private fun wordsCard(words: List<VocabularyWord>) = ui.card().apply {
        words.forEachIndexed { index, word ->
            if (index > 0) addView(ui.divider())
            addView(wordRow(word))
        }
    }

    /** 一行：译词与带它上屏的中文，下面次数与最近日期；右边等级与 认识 / 眼熟 / 生词 小标签。点开看详情。 */
    private fun wordRow(word: VocabularyWord) = LinearLayout(activity).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        background = ui.pressable()
        minimumHeight = ui.dp(60)
        setPadding(ui.dp(16), ui.dp(10), ui.dp(16), ui.dp(10))
        setOnClickListener { showWord(word) }
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.BOTTOM
                addView(ui.text(word.word, 17f, colors.text, bold = true))
                if (word.source.isNotEmpty()) {
                    addView(ui.text(word.source, 14f, colors.secondary).apply {
                        maxLines = 1
                        ellipsize = TextUtils.TruncateAt.END
                        setPadding(ui.dp(8), 0, 0, ui.dp(1))
                    })
                }
            })
            addView(ui.text(
                activity.getString(R.string.vocabulary_seen_times, word.seen) + " · " + word.last.drop(5),
                12f, colors.secondary,
            ).apply { setPadding(0, ui.dp(4), 0, 0) })
        }, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
        if (word.level.isNotEmpty()) addView(tag(word.level, colors.secondary, colors.fill))
        val (label, color) = when {
            word.known -> R.string.vocabulary_known to colors.brand
            word.fresh -> R.string.vocabulary_fresh to colors.fresh
            else -> R.string.vocabulary_familiar to colors.secondary
        }
        addView(tag(label, color, color and 0x00FFFFFF or 0x24000000))
    }

    private fun tag(label: Int, color: Int, fill: Int) = tag(activity.getString(label), color, fill)

    private fun tag(label: String, color: Int, fill: Int) = ui.text(label, 12f, color).apply {
        background = ui.rounded(fill, 10f)
        setPadding(ui.dp(8), ui.dp(3), ui.dp(8), ui.dp(3))
        layoutParams = LinearLayout.LayoutParams(WRAP_CONTENT, WRAP_CONTENT).apply { marginStart = ui.dp(8) }
    }

    /** 词的详情：中文、例句、几个次数；按钮是 认识了 / 不认识了、删除（再确认一次）。 */
    private fun showWord(word: VocabularyWord) {
        val lines = buildList {
            if (word.source.isNotEmpty()) add(word.source)
            if (word.example.isNotEmpty()) add(activity.getString(R.string.vocabulary_example, word.example))
            val counts = buildList {
                add(activity.getString(R.string.vocabulary_seen_times, word.seen))
                add(activity.getString(R.string.vocabulary_commits, word.committed))
                if (word.used > 0) add(activity.getString(R.string.vocabulary_used, word.used))
                add(activity.getString(R.string.vocabulary_last, word.last))
            }
            add(counts.joinToString(" · "))
        }
        AlertDialog.Builder(activity)
            .setTitle(if (word.level.isEmpty()) word.word else "${word.word}  ${word.level}")
            .setMessage(lines.joinToString("\n\n"))
            .setPositiveButton(if (word.known) R.string.vocabulary_unmark else R.string.vocabulary_mark) { _, _ ->
                refill(Vocabulary.setKnown(activity, query, word.word, !word.known))
            }
            .setNeutralButton(R.string.vocabulary_remove) { _, _ -> confirmRemove(word) }
            .setNegativeButton(R.string.close, null)
            .show()
    }

    private fun confirmRemove(word: VocabularyWord) {
        AlertDialog.Builder(activity)
            .setMessage(activity.getString(R.string.vocabulary_remove_confirm, word.word))
            .setPositiveButton(R.string.vocabulary_remove) { _, _ -> refill(Vocabulary.remove(activity, query, word.word)) }
            .setNegativeButton(android.R.string.cancel, null)
            .show()
    }

    companion object {
        private const val SEARCH_DELAY_MS = 250L
    }
}
