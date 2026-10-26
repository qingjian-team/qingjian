package app.qingjian.inputmethod.settings.home

import android.app.Activity
import android.app.AlertDialog
import android.os.Handler
import android.os.Looper
import android.text.Editable
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
import app.qingjian.inputmethod.theme.AppColors

/**
 * 生词本 tab：打字时候选旁见过的学习语言译词。顶上学习语言与三个汇总数字，下面是搜索框和按筛选与等级列出的词（最近见到的在前），
 * 点一个词弹出详情，可标认识或删除（同 PC 生词本的「词表」，时间线不做）。改了只重填这一列，滚动位置不丢。
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

    private fun signature(vocabulary: Vocabulary) = Triple(vocabulary.language, vocabulary.seen, vocabulary.matched)

    override fun stale() = signature(load()) != shown

    /** 换了学习语言后原来选的等级可能没有了，退回全部等级。 */
    private fun load(): Vocabulary {
        val vocabulary = Vocabulary.load(activity, query)
        if (query.level.isEmpty() || query.level in vocabulary.levels) return vocabulary
        query = query.copy(level = "")
        return Vocabulary.load(activity, query)
    }

    override fun header(): View = ui.pageTitle(R.string.tab_vocabulary)

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
        addView(languageRow(vocabulary))
        if (vocabulary.off) {
            addView(messageCard(activity.getString(R.string.vocabulary_off)))
            return
        }
        addView(summaryCard(vocabulary))
        (search.parent as? ViewGroup)?.removeView(search)
        addView(search, spaced(16))
        addView(chips(Filter.entries.map { filterLabel(it) to (it == query.filter) }) { index ->
            reload(query.copy(filter = Filter.entries[index]))
        }, spaced(12))
        if (vocabulary.levels.isNotEmpty()) {
            val levels = listOf("") + vocabulary.levels
            addView(chips(levels.map { (it.ifEmpty { activity.getString(R.string.vocabulary_all_levels) }) to (it == query.level) }) { index ->
                reload(query.copy(level = levels[index]))
            }, spaced(10))
        }
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            results = this
            fillResults(vocabulary)
        })
    }

    private fun LinearLayout.fillResults(vocabulary: Vocabulary) {
        addView(ui.text(activity.getString(R.string.vocabulary_count, vocabulary.matched), 15f, colors.text, bold = true).apply {
            setPadding(ui.dp(4), ui.dp(20), 0, ui.dp(10))
        })
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
        background = ui.rounded(colors.surface, 20f)
        setPadding(ui.dp(18), ui.dp(10), ui.dp(18), ui.dp(10))
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

    /** 一行说明，右边是学习语言，点开可换（与键盘设置里的同一项）。 */
    private fun languageRow(vocabulary: Vocabulary) = LinearLayout(activity).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        setPadding(ui.dp(4), 0, 0, ui.dp(14))
        addView(ui.text(activity.getString(R.string.vocabulary_intro), 12.5f, colors.secondary), LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
        addView(ui.text(SettingChoices.learningLanguageName(vocabulary.language) + " ⌄", 13f, colors.brand, bold = true).apply {
            background = ui.rounded(colors.brandSoft, 14f)
            setPadding(ui.dp(12), ui.dp(6), ui.dp(12), ui.dp(6))
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

    private fun summaryCard(vocabulary: Vocabulary) = ui.card(
        LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            setPadding(0, ui.dp(20), 0, ui.dp(18))
            listOf(
                R.string.vocabulary_seen to vocabulary.seen,
                R.string.vocabulary_familiar to vocabulary.familiar,
                R.string.vocabulary_new to vocabulary.newThisWeek,
            ).forEach { (label, value) ->
                addView(LinearLayout(activity).apply {
                    orientation = LinearLayout.VERTICAL
                    gravity = Gravity.CENTER_HORIZONTAL
                    addView(ui.text(value.toString(), 26f, colors.text, bold = true).apply { gravity = Gravity.CENTER })
                    addView(ui.text(activity.getString(label), 12f, colors.secondary).apply {
                        gravity = Gravity.CENTER
                        setPadding(0, ui.dp(8), 0, 0)
                    })
                }, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
            }
        },
    )

    /** 一排可横滑的圆角选项，选中的填品牌色。 */
    private fun chips(items: List<Pair<String, Boolean>>, onPick: (Int) -> Unit) = HorizontalScrollView(activity).apply {
        isHorizontalScrollBarEnabled = false
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            items.forEachIndexed { index, (label, selected) ->
                addView(ui.text(label, 13.5f, if (selected) colors.onBrand else colors.text, bold = selected).apply {
                    background = ui.rounded(if (selected) colors.brand else colors.surface, 16f)
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

    /** 一行：左边译词（带 认识 / 生词 与等级小标签）与带它上屏的中文，右边上屏次数与最近日期；点开看详情。 */
    private fun wordRow(word: VocabularyWord) = LinearLayout(activity).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        background = ui.pressable()
        setPadding(ui.dp(18), ui.dp(14), ui.dp(18), ui.dp(14))
        setOnClickListener { showWord(word) }
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                addView(ui.text(word.word, 17f, if (word.known) colors.secondary else colors.text, bold = true))
                when {
                    word.known -> addView(tag(R.string.vocabulary_known, colors.brand, colors.brandSoft))
                    word.fresh -> addView(tag(R.string.vocabulary_fresh, FRESH, FRESH and 0x00FFFFFF or 0x24000000))
                }
                if (word.level.isNotEmpty()) addView(tag(word.level, colors.secondary, colors.divider))
            })
            if (word.source.isNotEmpty()) {
                addView(ui.text(word.source, 13f, colors.secondary).apply { setPadding(0, ui.dp(6), 0, 0) })
            }
        }, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.END
            addView(ui.text(activity.getString(R.string.vocabulary_commits, word.committed), 12f, colors.secondary).apply { gravity = Gravity.END })
            addView(ui.text(word.last.drop(5), 11f, colors.faint).apply {
                gravity = Gravity.END
                setPadding(0, ui.dp(6), 0, 0)
            })
        }, LinearLayout.LayoutParams(WRAP_CONTENT, WRAP_CONTENT))
    }

    private fun tag(label: Int, color: Int, fill: Int) = tag(activity.getString(label), color, fill)

    private fun tag(label: String, color: Int, fill: Int) = ui.text(label, 10.5f, color, bold = true).apply {
        background = ui.rounded(fill, 6f)
        setPadding(ui.dp(6), ui.dp(3), ui.dp(6), ui.dp(3))
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
        /** 生词标签的橙色，同 [AppColors] 的橙色色块。 */
        private const val FRESH = 0xFFFF9443.toInt()

        private const val SEARCH_DELAY_MS = 250L
    }
}
