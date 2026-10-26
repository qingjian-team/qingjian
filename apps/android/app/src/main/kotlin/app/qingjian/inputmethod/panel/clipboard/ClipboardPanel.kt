package app.qingjian.inputmethod.panel.clipboard

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import android.graphics.Typeface
import android.text.Layout
import android.text.SpannableString
import android.text.Spanned
import android.text.StaticLayout
import android.text.TextPaint
import android.text.TextUtils
import android.text.style.ForegroundColorSpan
import android.text.style.StyleSpan
import app.qingjian.inputmethod.core.store.ClipItem
import app.qingjian.inputmethod.core.store.ClipboardStore
import app.qingjian.inputmethod.core.store.CodeView
import app.qingjian.inputmethod.core.store.PhraseItem
import app.qingjian.inputmethod.core.store.PhrasePage
import app.qingjian.inputmethod.core.store.PhraseRules
import app.qingjian.inputmethod.keyboard.Haptics
import app.qingjian.inputmethod.panel.PanelActions
import app.qingjian.inputmethod.panel.base.PanelIcons.Icon
import app.qingjian.inputmethod.panel.base.PanelView
import app.qingjian.inputmethod.panel.base.Tabs
import app.qingjian.inputmethod.settings.Settings
import app.qingjian.inputmethod.theme.Palette

/**
 * 剪贴板与常用语：表头两个页签，下面一张张圆角卡片，点卡片上屏，点右边 ⌄ 展开。
 * - 剪贴板：展开后 + 常用语 / 拆词 / 删除；右上角清空（点两次）。文字原样显示，像代码的用等宽字体、不重排；图片显示缩略图，长按看大图。
 * - 常用语：底下一排分组，展开后 编辑 / 移至首位 / 删除；右上角 + 新加一条。
 * 数据在 Rust 侧（[ClipboardStore]），这里每次改完拿回整份列表重画。
 */
@SuppressLint("ViewConstructor")
class ClipboardPanel(
    context: Context,
    palette: Palette,
    actions: PanelActions,
    headerHeight: Float,
    phrasesFirst: Boolean,
) : PanelView(context, palette, actions, headerHeight) {
    private val tabs = Tabs(ink, scrollable(true) { _, _ -> false }, Tabs.Style.UNDERLINE).apply {
        labels = listOf("剪贴板", "常用语")
        selected = if (phrasesFirst) PHRASES else CLIPS
    }
    private val showingPhrases get() = tabs.selected == PHRASES

    private var clips: List<ClipItem> = ClipboardStore.list(context, actions.clipboardLimit)
    private var page: PhrasePage = ClipboardStore.phrases(context)

    private val groups = Tabs(ink, scrollable(true) { x, y -> showingPhrases && y > groupTop() }, Tabs.Style.PILL).apply {
        labels = page.groups
        selected = page.groups.indexOf(lastGroup).coerceAtLeast(0)
    }

    private val list = RectF()
    private val listScroll = scrollable(false) { x, y -> list.contains(x, y) }

    /** 展开着的那一张（列表里的下标），-1 是都收着。 */
    private var expanded = -1

    /** 剪贴板「清空」点了第一下，等第二下确认。 */
    private var clearArmed = false

    /** 设置里的「代码高亮」；面板每次打开读一次。 */
    private val highlight = Settings.load(context).codeHighlight

    /** 数字的蓝：调色板里没有，按深浅色各取一个。 */
    private val numberColor = if (palette.dark) {
        0xFF7FA8F0.toInt()
    } else {
        0xFF2F6FDB.toInt()
    }

    private val textPaint = TextPaint(Paint.ANTI_ALIAS_FLAG)
    private val card = RectF()
    private val layouts = HashMap<String, StaticLayout>()
    private val thumbnails = HashMap<String, Bitmap?>()
    private val imagePaint = Paint(Paint.FILTER_BITMAP_FLAG)

    /** 长按打开的大图（图片完整路径），null 是没开。 */
    private var preview: String? = null
    private var previewDecoded: Pair<String, Bitmap?>? = null

    /** 每张卡片在列表内容里的上下边（不含滚动），画与点共用。 */
    private var tops = FloatArray(0)
    private var bottoms = FloatArray(0)

    private fun shownPhrases(): List<PhraseItem> {
        val group = page.groups.getOrNull(groups.selected) ?: PhraseRules.defaultGroup
        return page.phrases.filter { it.group == group }
    }

    private val count get() = if (showingPhrases) shownPhrases().size else clips.size

    private fun groupTop() = height - ink.dp(GROUP_BAR_DP)

    override fun onDraw(canvas: Canvas) {
        drawHeader(canvas, tabs, if (showingPhrases) Icon.PLUS else Icon.TRASH)
        val summary = if (showingPhrases) {
            "共 ${page.phrases.size} 条常用语，打输入码可直接出"
        } else if (clearArmed) {
            "再点一次右上角清空全部剪贴板"
        } else {
            "共 ${clips.size}/${actions.clipboardLimit} 条"
        }
        ink.text(canvas, summary, 12.5f, palette.gloss, width / 2f, headerHeight + ink.dp(SUMMARY_DP) / 2)
        val listBottom = if (showingPhrases) groupTop() else height - ink.dp(6f)
        list.set(0f, headerHeight + ink.dp(SUMMARY_DP), width.toFloat(), listBottom)
        arrange()
        canvas.save()
        canvas.clipRect(list)
        if (count == 0) {
            ink.text(
                canvas, if (showingPhrases) "这组还没有常用语，点右上角 + 加一条" else "复制的文字和图片会出现在这里",
                14f, palette.gloss, width / 2f, list.top + ink.dp(60f),
            )
        }
        for (i in 0 until count) {
            val top = list.top + tops[i] - listScroll.offset
            val bottom = list.top + bottoms[i] - listScroll.offset
            if (bottom < list.top || top > list.bottom) continue
            drawCard(canvas, i, top, bottom)
        }
        canvas.restore()
        if (showingPhrases) {
            groups.setBounds(ink.dp(6f), groupTop(), width - ink.dp(6f), height.toFloat() - ink.dp(4f))
            groups.draw(canvas)
        }
        preview?.let { drawPreview(canvas, it) }
    }

    /** 长按图片的大图：盖住整个面板，图按比例放到最大，底下 关闭 / 发送。 */
    private fun drawPreview(canvas: Canvas, path: String) {
        canvas.drawColor(palette.board)
        val barTop = height - ink.dp(PREVIEW_BAR_DP)
        val bitmap = previewBitmap(path)
        if (bitmap == null) {
            ink.text(canvas, "图片读不出来了", 15f, palette.gloss, width / 2f, barTop / 2)
        } else {
            val boxW = width - ink.dp(20f)
            val boxH = barTop - ink.dp(20f)
            val scale = minOf(boxW / bitmap.width, boxH / bitmap.height)
            val w = bitmap.width * scale
            val h = bitmap.height * scale
            card.set((width - w) / 2, (barTop - h) / 2, (width + w) / 2, (barTop + h) / 2)
            canvas.drawBitmap(bitmap, null, card, imagePaint)
        }
        val each = (width - ink.dp(20f)) / PREVIEW_BUTTONS.size
        PREVIEW_BUTTONS.forEachIndexed { i, label ->
            card.set(ink.dp(10f) + each * i + ink.dp(4f), barTop + ink.dp(8f), ink.dp(10f) + each * (i + 1) - ink.dp(4f), height - ink.dp(8f))
            val primary = i == PREVIEW_BUTTONS.lastIndex
            ink.round(canvas, card, 12f, if (primary) palette.brand else palette.surface)
            ink.text(canvas, label, 15f, if (primary) palette.onBrand else palette.text, card.centerX(), card.centerY(), bold = primary)
        }
    }

    /** 大图按面板大小降采样解码，只留当前这一张。 */
    private fun previewBitmap(path: String): Bitmap? {
        previewDecoded?.let { (decodedPath, bitmap) -> if (decodedPath == path) return bitmap }
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeFile(path, bounds)
        var sample = 1
        while (bounds.outWidth / (sample * 2) >= width && bounds.outHeight / (sample * 2) >= height) sample *= 2
        val bitmap = BitmapFactory.decodeFile(path, BitmapFactory.Options().apply { inSampleSize = sample })
        previewDecoded = path to bitmap
        return bitmap
    }

    override fun onHold(x: Float, y: Float, rawX: Float) {
        if (showingPhrases || preview != null || !list.contains(x, y)) return
        val contentY = y - list.top + listScroll.offset
        val i = (0 until count).firstOrNull { contentY >= tops[it] && contentY < bottoms[it] } ?: return
        preview = clips[i].image ?: return
        Haptics.tap(context)
        invalidate()
    }

    override fun onBack() {
        if (preview != null) {
            closePreview()
        } else {
            super.onBack()
        }
    }

    private fun closePreview() {
        preview = null
        previewDecoded = null
        invalidate()
    }

    /** 量出每张卡片的高；展开的那张多一行操作。 */
    private fun arrange() {
        tops = FloatArray(count)
        bottoms = FloatArray(count)
        var y = ink.dp(2f)
        for (i in 0 until count) {
            tops[i] = y
            y += contentHeight(i) + ink.dp(CARD_PADDING_DP) * 2 + (if (i == expanded) ink.dp(ACTIONS_DP) else 0f)
            bottoms[i] = y
            y += ink.dp(CARD_GAP_DP)
        }
        listScroll.max = y - list.height()
    }

    private fun textWidth() = (width - ink.dp(CARD_MARGIN_DP) * 2 - ink.dp(CARD_PADDING_DP) * 2 - ink.dp(CHEVRON_DP)).toInt()

    private fun contentHeight(i: Int): Float {
        if (!showingPhrases) clips[i].image?.let { return ink.dp(IMAGE_DP) }
        return layout(i).height + (if (codeOf(i) != null) ink.dp(CODE_LINE_DP) else 0f)
    }

    /** 常用语的输入码（有才画一行）。 */
    private fun codeOf(i: Int): String? = if (showingPhrases) shownPhrases()[i].code.ifEmpty { null } else null

    private fun textOf(i: Int) = if (showingPhrases) shownPhrases()[i].text else clips[i].text

    /**
     * 卡片里的字：收着最多 3 行、展开最多 15 行；像代码的（Core 认，见 [CodeView]）等宽、不重排，开着「代码高亮」再按片段上色。
     * 只排前一段，长文不整篇排版。
     */
    private fun layout(i: Int): StaticLayout {
        val text = textOf(i)
        val open = i == expanded
        val key = "$open|${textWidth()}|$text"
        return layouts.getOrPut(key) {
            val shown = text.take(if (open) 3000 else 600)
            val view = CodeView.of(shown)
            textPaint.textSize = ink.dp(if (view.code) 13f else 15.5f)
            textPaint.typeface = if (view.code) Typeface.MONOSPACE else Typeface.DEFAULT
            textPaint.color = palette.text
            val styled: CharSequence = if (view.code && highlight) colored(shown, view) else shown
            StaticLayout.Builder.obtain(styled, 0, styled.length, TextPaint(textPaint), textWidth().coerceAtLeast(1))
                .setAlignment(Layout.Alignment.ALIGN_NORMAL)
                .setMaxLines(if (open) 15 else 3)
                .setEllipsize(TextUtils.TruncateAt.END)
                .setLineSpacing(0f, 1.15f)
                .build()
        }
    }

    /** 按 Core 切的片段上色：关键字品牌色、字符串橙、注释灰（斜体）、数字蓝。 */
    private fun colored(text: String, view: CodeView): CharSequence {
        val styled = SpannableString(text)
        for (span in view.spans) {
            if (span.start < 0 || span.end > text.length || span.start >= span.end) continue
            val color = when (span.kind) {
                "keyword" -> palette.brand
                "string" -> palette.fresh
                "comment" -> palette.gloss
                else -> numberColor
            }
            styled.setSpan(ForegroundColorSpan(color), span.start, span.end, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE)
            if (span.kind == "comment") styled.setSpan(StyleSpan(Typeface.ITALIC), span.start, span.end, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE)
        }
        return styled
    }

    private fun drawCard(canvas: Canvas, i: Int, top: Float, bottom: Float) {
        val margin = ink.dp(CARD_MARGIN_DP)
        val padding = ink.dp(CARD_PADDING_DP)
        card.set(margin, top, width - margin, bottom)
        ink.round(canvas, card, 12f, palette.surface)
        val image = if (showingPhrases) null else clips[i].image
        if (image != null) {
            thumbnail(image)?.let { bitmap ->
                val h = ink.dp(IMAGE_DP)
                val w = (bitmap.width * h / bitmap.height).coerceAtMost(textWidth().toFloat())
                card.set(margin + padding, top + padding, margin + padding + w, top + padding + h)
                canvas.drawBitmap(bitmap, null, card, null)
            } ?: ink.textStart(canvas, "[图片]", 15f, palette.gloss, margin + padding, top + padding + ink.dp(IMAGE_DP) / 2, width - margin)
        } else {
            canvas.save()
            canvas.translate(margin + padding, top + padding)
            layout(i).draw(canvas)
            canvas.restore()
            codeOf(i)?.let { code ->
                val y = top + padding + layout(i).height + ink.dp(CODE_LINE_DP) / 2
                ink.textStart(canvas, "输入码  $code", 12.5f, palette.gloss, margin + padding, y, width - margin)
            }
        }
        val open = i == expanded
        ink.icon(canvas, if (open) Icon.COLLAPSE else Icon.EXPAND, width - margin - ink.dp(CHEVRON_DP) / 2, top + padding + ink.dp(11f), 20f, palette.gloss)
        if (open) drawActions(canvas, bottom - ink.dp(ACTIONS_DP), bottom)
    }

    private fun actionLabels(): List<Pair<Icon, String>> = if (showingPhrases) {
        listOf(Icon.PEN to "编辑", Icon.TOP to "移至首位", Icon.TRASH to "删除")
    } else if (clips.getOrNull(expanded)?.image != null) {
        listOf(Icon.TRASH to "删除")
    } else {
        listOf(Icon.PLUS to "常用语", Icon.SPLIT to "拆词", Icon.TRASH to "删除")
    }

    private fun actionWidth() = ink.dp(96f)

    private fun drawActions(canvas: Canvas, top: Float, bottom: Float) {
        val cy = (top + bottom) / 2 - ink.dp(4f)
        var x = ink.dp(CARD_MARGIN_DP + CARD_PADDING_DP)
        for ((icon, label) in actionLabels()) {
            ink.icon(canvas, icon, x + ink.dp(9f), cy, 17f, palette.gloss)
            ink.textStart(canvas, label, 13.5f, palette.text, x + ink.dp(22f), cy, x + actionWidth())
            x += actionWidth()
        }
    }

    private fun thumbnail(path: String): Bitmap? = thumbnails.getOrPut(path) {
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeFile(path, bounds)
        var sample = 1
        while (bounds.outHeight / (sample * 2) >= ink.dp(IMAGE_DP)) sample *= 2
        BitmapFactory.decodeFile(path, BitmapFactory.Options().apply { inSampleSize = sample })
    }

    override fun onTap(x: Float, y: Float) {
        preview?.let { path ->
            val send = y > height - ink.dp(PREVIEW_BAR_DP) && x > width / 2f
            closePreview()
            if (send) actions.commitImage(path)
            return
        }
        when {
            y < headerHeight && isRightIcon(x, y) -> onRightIcon()
            y < headerHeight -> tabs.indexAt(x)?.let(::switchTab)
            showingPhrases && y > groupTop() -> groups.indexAt(x)?.let { index ->
                groups.selected = index
                lastGroup = page.groups[index]
                collapse()
            }
            list.contains(x, y) -> tapList(x, y - list.top + listScroll.offset)
        }
        if (!(y < headerHeight && isRightIcon(x, y))) clearArmed = false
    }

    private fun switchTab(index: Int) {
        if (index == tabs.selected) return
        tabs.selected = index
        collapse()
    }

    private fun collapse() {
        expanded = -1
        listScroll.reset()
        layouts.clear()
    }

    private fun onRightIcon() {
        if (showingPhrases) {
            actions.editPhrase(null, page.groups.getOrNull(groups.selected) ?: PhraseRules.defaultGroup)
        } else if (clips.isNotEmpty()) {
            if (clearArmed) {
                clips = ClipboardStore.clear(context, actions.clipboardLimit)
                clearArmed = false
                collapse()
            } else {
                clearArmed = true
            }
        }
    }

    /** [y] 是列表内容里的位置（已加滚动）。 */
    private fun tapList(x: Float, y: Float) {
        val i = (0 until count).firstOrNull { y >= tops[it] && y < bottoms[it] } ?: return
        val chevron = x > width - ink.dp(CARD_MARGIN_DP + CHEVRON_DP + 8f) && y < tops[i] + ink.dp(CARD_PADDING_DP + 30f)
        val inActions = i == expanded && y > bottoms[i] - ink.dp(ACTIONS_DP)
        when {
            chevron -> {
                expanded = if (expanded == i) -1 else i
                layouts.clear()
            }
            inActions -> onAction(i, ((x - ink.dp(CARD_MARGIN_DP + CARD_PADDING_DP)) / actionWidth()).toInt())
            showingPhrases -> actions.commitText(shownPhrases()[i].text)
            else -> clips[i].let { clip -> clip.image?.let(actions::commitImage) ?: actions.commitText(clip.text) }
        }
    }

    private fun onAction(i: Int, action: Int) {
        val labels = actionLabels()
        val label = labels.getOrNull(action)?.second ?: return
        if (showingPhrases) {
            val phrase = shownPhrases()[i]
            when (label) {
                "编辑" -> actions.editPhrase(phrase, phrase.group)
                "移至首位" -> reloadPhrases(ClipboardStore.phraseToTop(context, phrase.index))
                "删除" -> reloadPhrases(ClipboardStore.removePhrase(context, phrase.index))
            }
            return
        }
        when (label) {
            "常用语" -> {
                val group = lastGroup
                reloadPhrases(ClipboardStore.addPhrase(context, clips[i].text, group, ""))
                actions.toast("已加入常用语「$group」")
            }
            "拆词" -> actions.openPieces(clips[i].text)
            "删除" -> {
                clips = ClipboardStore.remove(context, i, actions.clipboardLimit)
                collapse()
            }
        }
    }

    private fun reloadPhrases(page: PhrasePage) {
        this.page = page
        groups.labels = page.groups
        groups.selected = page.groups.indexOf(lastGroup).coerceAtLeast(0)
        actions.phrasesChanged()
        collapse()
    }

    companion object {
        private const val CLIPS = 0
        private const val PHRASES = 1
        private const val SUMMARY_DP = 28f
        private const val GROUP_BAR_DP = 48f
        private const val CARD_MARGIN_DP = 10f
        private const val CARD_PADDING_DP = 14f
        private const val CARD_GAP_DP = 8f
        private const val CHEVRON_DP = 32f
        private const val ACTIONS_DP = 40f
        private const val IMAGE_DP = 96f
        private const val CODE_LINE_DP = 24f
        private const val PREVIEW_BAR_DP = 60f
        private val PREVIEW_BUTTONS = listOf("关闭", "发送")

        /** 常用语停在哪一组：关了面板再开还在这组；加完常用语跳到它所在的组。 */
        var lastGroup = PhraseRules.defaultGroup
    }
}
