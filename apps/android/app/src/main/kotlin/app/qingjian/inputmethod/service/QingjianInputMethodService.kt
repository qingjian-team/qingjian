package app.qingjian.inputmethod.service

import android.content.ClipData
import android.content.ClipDescription
import android.content.ClipboardManager
import android.content.Intent
import android.content.SharedPreferences
import android.content.res.Configuration
import android.inputmethodservice.InputMethodService
import android.view.KeyEvent
import android.view.View
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import android.view.inputmethod.InputContentInfo
import android.view.inputmethod.InputMethodManager
import android.widget.Toast
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.candidates.CandidateBarView
import app.qingjian.inputmethod.clipboard.ClipboardImageProvider
import app.qingjian.inputmethod.clipboard.sync.ClipboardSync
import app.qingjian.inputmethod.clipboard.ClipboardWatcher
import app.qingjian.inputmethod.clipboard.sync.NeonEdgeView
import org.json.JSONObject
import app.qingjian.inputmethod.contacts.ContactsSync
import app.qingjian.inputmethod.core.AppLog
import app.qingjian.inputmethod.core.Engine
import app.qingjian.inputmethod.core.InputOptions
import app.qingjian.inputmethod.core.frame.Candidate
import app.qingjian.inputmethod.core.store.ClipboardStore
import app.qingjian.inputmethod.core.store.PhraseItem
import app.qingjian.inputmethod.core.store.PhraseRules
import app.qingjian.inputmethod.core.store.Shuangpin
import app.qingjian.inputmethod.keyboard.KeyboardView
import app.qingjian.inputmethod.keyboard.ShiftState
import app.qingjian.inputmethod.keyboard.layout.Key
import app.qingjian.inputmethod.keyboard.layout.KeyboardLayout
import app.qingjian.inputmethod.panel.PanelActions
import app.qingjian.inputmethod.panel.PanelKind
import app.qingjian.inputmethod.panel.VoicePanel
import app.qingjian.inputmethod.panel.clipboard.ClipboardPanel
import app.qingjian.inputmethod.panel.editor.FloatingEditor
import app.qingjian.inputmethod.panel.editor.PhraseEditor
import app.qingjian.inputmethod.service.input.Composer
import app.qingjian.inputmethod.service.input.Language
import app.qingjian.inputmethod.service.input.enterLabel
import app.qingjian.inputmethod.service.input.wantsDirectInput
import app.qingjian.inputmethod.service.input.wantsPlainPunctuation
import app.qingjian.inputmethod.service.input.wantsNumbers
import app.qingjian.inputmethod.service.input.wantsPrivacy
import app.qingjian.inputmethod.settings.BottomAction
import app.qingjian.inputmethod.settings.GameKeyboard
import app.qingjian.inputmethod.settings.Settings
import app.qingjian.inputmethod.settings.ToolbarTool
import app.qingjian.inputmethod.settings.home.MainActivity
import app.qingjian.inputmethod.settings.page.CloudSettingsActivity
import app.qingjian.inputmethod.settings.page.VoiceSettingsActivity
import app.qingjian.inputmethod.theme.Palette
import app.qingjian.inputmethod.translate.QuickTranslate
import app.qingjian.inputmethod.translate.TranslateBar
import app.qingjian.inputmethod.voice.VoiceInput

/** 输入法服务：管生命周期、设置与视图，输入逻辑在 [Composer]。 */
class QingjianInputMethodService :
    InputMethodService(), Composer.Host, CandidateBarView.Listener, KeyboardView.Listener, PanelActions, VoiceInput.Host,
    GameMode.Host {
    private lateinit var engine: Engine
    private lateinit var composer: Composer
    private var panel: InputPanel? = null

    private val game = GameMode(this, this)

    private lateinit var cloud: CloudServices

    /** 当前输入框的回车键文字，见 [enterLabel]。 */
    private var enterLabel: String? = null

    /** 候选条上最新的拼音与候选，展开候选时交给 [CandidateGrid][app.qingjian.inputmethod.candidates.CandidateGrid]。 */
    private var shownPreedit = ""
    private var shownCandidates: List<Candidate> = emptyList()

    /** 拖动调高度开始时的百分比；拖动中不为 null。 */
    private var resizeFrom: Int? = null

    /** 就地改设置会重建键盘，重建完重新打开这个面板。 */
    private var reopenPanel: PanelKind? = null

    private val clipboardWatcher by lazy { ClipboardWatcher(this) }

    private var neon: NeonEdgeView? = null

    /** 同步的事件：键盘边缘亮灯，收到内容后开着的剪贴板面板重读。 */
    private val syncListener = object : ClipboardSync.Listener {
        override fun onSyncEvent(type: String, event: JSONObject) {
            val glow = neon?.takeIf { ClipboardSync.neonEnabled(this@QingjianInputMethodService) }
            when (type) {
                "started" -> glow?.start()
                "progress" -> glow?.speed(event.optLong("speed"))
                "finished" -> {
                    glow?.finish(true)
                    panel?.refreshClipboard()
                }
                "failed" -> glow?.finish(false)
            }
        }
    }

    /** 浮在键盘上方接管打字的编辑条（常用语编辑框、快捷翻译）；null 是没开。 */
    private var editor: FloatingEditor? = null

    private lateinit var quickTranslate: QuickTranslate

    /** 语音输入：长按空格、工具栏话筒、全面屏底栏的语音键都进它。 */
    private lateinit var voice: VoiceInput

    /** 返回键关了面板，它的抬起也要吞掉。 */
    private var backConsumed = false

    /** 自己写设置（拖完高度）时不重建键盘。 */
    private var writingSettings = false

    override fun writeSettings(block: () -> Unit) {
        writingSettings = true
        try {
            block()
        } finally {
            writingSettings = false
        }
    }

    /** 设置页改了就重建键盘，开着键盘能马上看到。 */
    private val settingsListener = SharedPreferences.OnSharedPreferenceChangeListener { _, _ ->
        contacts.sync()
        if (!writingSettings) setInputView(onCreateInputView())
    }

    /** 通讯录里的名字（设置里开了才读）。 */
    private lateinit var contacts: ContactsSync

    override fun onCreate() {
        super.onCreate()
        AppLog.info("ime create")
        engine = Engine(this) {
            composer.onEngineReady()
            cloud.sync()
            cloud.syncVoice()
            contacts.sync()
        }
        contacts = ContactsSync(this, engine)
        cloud = CloudServices(this, engine)
        composer = Composer(engine, this)
        quickTranslate = QuickTranslate(engine) { currentInputConnection }
        voice = VoiceInput(this, engine, this)
        Settings.load(this).let { settings ->
            composer.restoreLanguages(
                Language.of(settings.language, Language.CHINESE),
                Language.of(settings.partner, Language.ENGLISH),
            )
        }
        Settings.preferences(this).registerOnSharedPreferenceChangeListener(settingsListener)
        cloud.start()
        clipboardWatcher.onChanged = { panel?.refreshClipboard() }
        clipboardWatcher.start()
        ClipboardSync.addListener(syncListener)
        ClipboardSync.apply(this)
    }

    override fun onDestroy() {
        AppLog.info("ime destroy")
        clipboardWatcher.stop()
        contacts.stop()
        ClipboardSync.removeListener(syncListener)
        Settings.preferences(this).unregisterOnSharedPreferenceChangeListener(settingsListener)
        cloud.stop()
        engine.close()
        super.onDestroy()
    }

    override fun onCreateInputView(): View {
        // 键盘重建，开着的语音面板跟着没了
        voice.cancel()
        val target = game.target()
        AppLog.info("create input view${if (target != null) " (game keyboard)" else ""}")
        val gameSettings = target?.let { GameKeyboard.load(this) }
        val settings = Settings.load(this).let { settings ->
            gameSettings ?: return@let settings
            settings.copy(heightPercent = gameSettings.size.heightPercent, fullscreenAdapt = false)
        }
        composer.layout = settings.layout
        composer.traditional = settings.traditional
        composer.keepChinese = settings.keepChinese
        composer.shuangpinScheme = settings.shuangpinScheme
        composer.shuangpinRawPreedit = settings.shuangpinRawPreedit
        composer.wubiMixed = settings.wubiMixed
        composer.options = settings.inputOptions.let { options ->
            if (gameSettings?.glosses == false) options.copy(learningLanguage = InputOptions.LEARNING_OFF) else options
        }
        composer.englishCandidates = settings.englishCandidates
        engine.setAppearance(settings.theme, settings.textSize.scale)
        cloud.sync(settings)
        // 布局没变时 layout 的 setter 不会重设方案，双拼方案、混输这些改了也得补上
        composer.applyScheme()
        val panel = InputPanel(
            this, settings, this,
            stripSource = { highlighted, dark, visibleWidth -> engine.renderStrip(highlighted, dark, visibleWidth) },
            keyboardListener = this,
            actions = this,
            onBottomAction = ::onBottomAction,
            floating = target != null,
        )
        this.panel = panel
        showKeyboard()
        showSideItems(emptyList())
        reopenPanel?.let(panel::open)
        reopenPanel = null
        game.wrap(target, settings, gameSettings, panel.view)?.let { frame ->
            neon = null
            return frame
        }
        val glow = NeonEdgeView(this)
        neon = glow
        panel.addOverlay(glow)
        return panel.view
    }

    /** 游戏键盘不进横屏的全屏编辑（那会盖住游戏），其余照系统的判断。 */
    override fun onEvaluateFullscreenMode(): Boolean = game.target() == null && super.onEvaluateFullscreenMode()

    override fun onComputeInsets(outInsets: Insets) {
        super.onComputeInsets(outInsets)
        game.computeInsets(outInsets, window.window?.decorView?.height)
    }

    override val editorPackage: String? get() = currentInputEditorInfo?.packageName

    override fun rebuild() {
        closePanel()
        setInputView(onCreateInputView())
        updateFullscreenMode()
    }

    override fun hide() = onHideKeyboard()

    override val gameKeyboard: Boolean? get() = game.enabledHere

    override fun toggleGameKeyboard() = game.toggle()

    /** 深浅色跟着系统变。 */
    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        setInputView(onCreateInputView())
    }

    override fun onStartInput(attribute: EditorInfo, restarting: Boolean) {
        super.onStartInput(attribute, restarting)
        composer.directInput = attribute.wantsDirectInput()
        composer.plainPunctuation = attribute.wantsPlainPunctuation()
        composer.privateInput = attribute.wantsPrivacy()
        enterLabel = attribute.enterLabel()
        panel?.keyboard?.enterLabel = enterLabel
        if (!(restarting && composer.resume())) composer.abandon()
        if (attribute.wantsNumbers()) composer.showNumbers()
    }

    override fun onStartInputView(info: EditorInfo, restarting: Boolean) {
        super.onStartInputView(info, restarting)
        AppLog.info(
            "start input view: ${info.packageName}, inputType 0x${Integer.toHexString(info.inputType)}, " +
                "imeOptions 0x${Integer.toHexString(info.imeOptions)}, restarting $restarting",
        )
        game.rememberLandscapeApp(info.packageName)
        if (panel?.stale() == true || game.target() != game.packageName) {
            setInputView(onCreateInputView())
            updateFullscreenMode()
        }
        // 网络可能换了（「只在 Wi-Fi 下使用」）
        cloud.sync()
        // 上次读通讯录时系统给了空结果（「仅在使用中允许」时后台读不到），键盘弹出时再读
        contacts.retryIfEmpty()
        clipboardWatcher.capture()
        // 进程可能刚被系统杀过又拉起，或开关刚改过；顺便补一轮
        ClipboardSync.apply(this)
        ClipboardSync.changed()
    }

    /** 系统问「该不该显示键盘」：接着实体键盘时缺省不显示，有时候弹不出来先看这一条。 */
    override fun onEvaluateInputViewShown(): Boolean {
        val shown = super.onEvaluateInputViewShown()
        if (!shown) {
            val config = resources.configuration
            AppLog.warn("input view not shown: keyboard ${config.keyboard}, hardKeyboardHidden ${config.hardKeyboardHidden}")
        }
        return shown
    }

    override fun onWindowShown() {
        super.onWindowShown()
        AppLog.info("window shown")
    }

    override fun onWindowHidden() {
        super.onWindowHidden()
        AppLog.info("window hidden")
    }

    override fun onFinishInputView(finishingInput: Boolean) {
        super.onFinishInputView(finishingInput)
        voice.cancel()
        closeEditor()
        panel?.close()
        composer.abandon()
    }

    override fun onUpdateSelection(
        oldSelStart: Int, oldSelEnd: Int,
        newSelStart: Int, newSelEnd: Int,
        candidatesStart: Int, candidatesEnd: Int,
    ) {
        super.onUpdateSelection(
            oldSelStart, oldSelEnd, newSelStart, newSelEnd, candidatesStart, candidatesEnd,
        )
        if (editor != null) return
        val moved = newSelStart != candidatesEnd || newSelEnd != candidatesEnd
        if (candidatesEnd != -1 && moved) composer.onCursorMovedAway()
        if (candidatesEnd == -1 && newSelStart == newSelEnd) composer.refreshCompletions()
    }

    // 编辑条开着时打的字进编辑条
    override val connection: InputConnection? get() = editor?.connection ?: currentInputConnection

    override var shift: ShiftState
        get() = panel?.keyboard?.shift ?: ShiftState.OFF
        set(value) {
            panel?.keyboard?.shift = value
        }

    override fun sendKey(keyCode: Int) = editor?.onKey(keyCode) ?: sendDownUpKeyEvents(keyCode)

    override fun performEditorAction(): Boolean = editor == null && sendDefaultEditorAction(true)

    override fun showComposition(preedit: String, candidates: List<Candidate>) {
        shownPreedit = preedit
        shownCandidates = candidates
        panel?.bar?.show(preedit, candidates)
        if (panel?.openPanel == PanelKind.CANDIDATES) {
            if (candidates.isEmpty()) closePanel() else panel?.showCandidates(preedit, candidates, composer.singleChars)
        }
    }

    override fun onExpand() {
        openPanel(PanelKind.CANDIDATES)
        panel?.showCandidates(shownPreedit, shownCandidates, composer.singleChars)
    }

    override fun selectCandidate(index: Int) = composer.select(index)

    override fun toggleSingleChars() = composer.toggleSingleChars()

    override fun clearComposition() = composer.onKey(Key.Clear)

    override fun showKeyboard() {
        val panel = panel ?: return
        val keyboard = panel.keyboard
        val settings = panel.settings
        keyboard.chinese = composer.chinese
        keyboard.languageLabel = composer.language.key
        keyboard.languageHint = composer.partner.key
        keyboard.languageChoices = Language.entries.map { it.title }
        keyboard.languageChoice = composer.language.ordinal
        keyboard.enterLabel = editor?.enterLabel ?: enterLabel
        val fullWidth = composer.options.fullWidthPunctuation
        // 滑行只给拼音全键盘与英文键盘（常用语编辑条的英文键盘也算）：双拼、五笔、日语的字母不是这样拼词的
        keyboard.swipeLayout = composer.page == Composer.Page.LETTERS && (
            editor?.lettersOnly == true || (!composer.nineKey && !composer.zhuyin && !composer.shuangpin && !composer.wubi && !composer.japanese)
        )
        when {
            composer.page == Composer.Page.NUMBERS -> keyboard.setRows(KeyboardLayout.numbers(), side = true)
            composer.page == Composer.Page.SYMBOLS -> keyboard.setRows(KeyboardLayout.symbols(composer.chinese, fullWidth))
            // 常用语的输入码只收字母，九宫格、注音打不出来，换成英文全键盘
            editor?.lettersOnly == true -> keyboard.setRows(KeyboardLayout.qwerty(chinese = false))
            composer.nineKey -> keyboard.setRows(KeyboardLayout.nineKey(), side = true)
            composer.zhuyin -> keyboard.setRows(KeyboardLayout.zhuyin(fullWidth))
            composer.shuangpin -> keyboard.setRows(
                KeyboardLayout.shuangpin(
                    Shuangpin.keycaps(composer.shuangpinScheme), composer.shuangpinScheme, fullWidth,
                    hints = settings.shuangpinHints,
                ),
            )
            else -> keyboard.setRows(KeyboardLayout.qwerty(composer.chinese, composer.japanese, fullWidth))
        }
        showSideItems(emptyList())
    }

    /** 左列：数字键盘上是符号，九宫格组句中是同码拼音，平时是标点。 */
    override fun showSideItems(pinyinChoices: List<String>) {
        panel?.keyboard?.setSideItems(
            when {
                composer.page == Composer.Page.NUMBERS -> KeyboardLayout.NUMBER_SIDE
                else -> pinyinChoices.ifEmpty { KeyboardLayout.sidePunctuation(composer.options.fullWidthPunctuation) }
            },
        )
    }

    override fun onKey(key: Key) = composer.onKey(key)

    override fun onKeyChoice(text: String) = composer.onKeyChoice(text)

    override fun onLanguageChoice(index: Int) = composer.chooseLanguage(Language.entries[index])

    /** 语言换了：记进设置（自己写的不重建键盘），中英键跟着重画。 */
    override fun onLanguageChanged() {
        writeSettings {
            Settings.preferences(this).edit()
                .putString(Settings.KEY_LANGUAGE, composer.language.name)
                .putString(Settings.KEY_PARTNER, composer.partner.name)
                .commit()
        }
        showKeyboard()
    }

    override fun onCursorMove(steps: Int) = composer.moveCursor(steps)

    override fun onSideItem(item: String) = composer.onSideItem(item)

    override fun onSelect(index: Int) = composer.select(index)

    override fun onMoveCaret(index: Int) = composer.moveCaret(index)

    override fun onSelectTranslation(index: Int, sense: Int) = composer.selectTranslation(index, sense)

    override fun onForget(index: Int) = composer.forget(index)

    override fun showNotice(text: String) = toast(text)

    override fun openSymbolPanel() = openPanel(PanelKind.SYMBOLS)

    override fun onToolbar(tool: ToolbarTool) = when (tool) {
        ToolbarTool.EMOJI -> openPanel(PanelKind.EMOJI)
        ToolbarTool.KEYBOARDS -> openPanel(PanelKind.KEYBOARDS)
        ToolbarTool.VOICE -> voiceInput()
        ToolbarTool.SYMBOLS -> openPanel(PanelKind.SYMBOLS)
        ToolbarTool.CLIPBOARD -> clipboard()
        ToolbarTool.TRANSLATE -> toggleTranslate()
        ToolbarTool.RESIZE -> startResize()
        ToolbarTool.PHRASES -> openPanel(PanelKind.PHRASES)
        ToolbarTool.SWITCH_INPUT_METHOD -> switchInputMethod()
        ToolbarTool.SETTINGS -> openSettings()
    }

    override fun openFunctions() = openPanel(PanelKind.FUNCTIONS)

    override fun openPanel(kind: PanelKind) {
        panel?.open(kind)
    }

    override fun closePanel() {
        // 返回键关的是语音面板：这一段不要了
        if (voice.active) {
            voice.cancel()
            return
        }
        val candidates = panel?.openPanel == PanelKind.CANDIDATES
        panel?.close()
        // 单字是展开面板里的筛选：收起就回到全部候选
        if (candidates && composer.singleChars) composer.toggleSingleChars()
    }

    override fun commitText(text: String) = composer.commitPanelText(text)

    override fun deleteBackward() = composer.onKey(Key.Backspace)

    override fun startResize() {
        closePanel()
        panel?.bar?.startResize()
    }

    override fun switchInputMethod() {
        getSystemService(InputMethodManager::class.java).showInputMethodPicker()
    }

    override fun chooseLayout(layout: Settings.Layout) {
        closePanel()
        composer.chooseLanguage(Language.CHINESE)
        Settings.preferences(this).edit().putString(Settings.KEY_LAYOUT, layout.name).apply()
    }

    override fun chooseEnglish() {
        closePanel()
        composer.chooseLanguage(Language.ENGLISH)
    }

    override fun chooseJapanese() {
        closePanel()
        composer.chooseLanguage(Language.JAPANESE)
    }

    override val english: Boolean get() = !composer.chinese && !composer.japanese

    override val japanese: Boolean get() = composer.japanese

    override fun setToggle(key: String, on: Boolean) {
        reopenPanel = panel?.openPanel
        Settings.preferences(this).edit().putBoolean(key, on).apply()
    }

    override fun setChoice(key: String, value: String) {
        reopenPanel = panel?.openPanel
        Settings.preferences(this).edit().putString(key, value).apply()
    }

    /** 面板开着时系统返回键先关面板。 */
    override fun onKeyDown(keyCode: Int, event: KeyEvent): Boolean {
        if (keyCode == KeyEvent.KEYCODE_BACK && event.repeatCount == 0 && panel?.openPanel != null) {
            closePanel()
            backConsumed = true
            return true
        }
        return super.onKeyDown(keyCode, event)
    }

    override fun onKeyUp(keyCode: Int, event: KeyEvent): Boolean {
        if (keyCode == KeyEvent.KEYCODE_BACK && backConsumed) {
            backConsumed = false
            return true
        }
        return super.onKeyUp(keyCode, event)
    }

    override fun openSettings() {
        startActivity(
            Intent(this, MainActivity::class.java)
                .putExtra(MainActivity.EXTRA_TAB, MainActivity.TAB_SETTINGS)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP),
        )
    }

    private fun onHideKeyboard() = requestHideSelf(0)

    private fun onBottomAction(action: BottomAction) = when (action) {
        BottomAction.SWITCH_INPUT_METHOD -> switchInputMethod()
        BottomAction.VOICE -> voiceInput()
        BottomAction.CLIPBOARD -> clipboard()
        BottomAction.HIDE_KEYBOARD -> onHideKeyboard()
        BottomAction.NONE -> Unit
    }

    /** 工具栏、全面屏底栏的剪贴板：打开剪贴板面板。 */
    private fun clipboard() = openPanel(PanelKind.CLIPBOARD)

    override val clipboardLimit: Int get() = Settings.load(this).clipboardLimit

    override fun openPieces(text: String) {
        panel?.open(PanelKind.PIECES, text)
    }

    override fun textPieces(text: String): List<String> = engine.textPieces(text)

    override fun phrasesChanged() = engine.reloadPhrases(ClipboardStore.userDir(this))

    override fun toast(message: String) = Toast.makeText(this, message, Toast.LENGTH_SHORT).show()

    override fun voiceNeedsSetup() = needsSetup(getString(R.string.voice_needs_setup), VoiceSettingsActivity::class.java)

    /**
     * 要先配好才能用的功能（语音、快捷翻译）：在工具栏的位置放一句提示，点它打开 [page]；
     * 不只弹 toast：toast 一闪就过，也没有去设置的路。键盘没建起来时退回 toast。
     */
    private fun needsSetup(message: String, page: Class<*>) {
        val bar = panel?.bar ?: return toast(message)
        closePanel()
        bar.showNotice(message) {
            startActivity(Intent(this, page).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        }
    }

    override fun composeText(text: String) {
        currentInputConnection?.setComposingText(text, 1)
    }

    override fun finishComposeText() {
        currentInputConnection?.finishComposingText()
    }

    override fun copyToClipboard(text: String) {
        getSystemService(ClipboardManager::class.java)?.setPrimaryClip(ClipData.newPlainText("青简", text))
        toast("已复制")
    }

    /**
     * 剪贴板里点了图片：输入框收图片就用 `commitContent` 发过去（对方经 [ClipboardImageProvider] 临时授权读），
     * 不收的就放回系统剪贴板，让用户长按粘贴。
     */
    override fun commitImage(path: String) {
        val uri = ClipboardImageProvider.uriOf(path)
        val mime = ClipboardImageProvider.mimeOf(path)
        val accepts = currentInputEditorInfo?.contentMimeTypes.orEmpty().any { ClipDescription.compareMimeTypes(mime, it) }
        val connection = currentInputConnection
        if (accepts && connection != null) {
            val info = InputContentInfo(uri, ClipDescription("图片", arrayOf(mime)), null)
            if (connection.commitContent(info, InputConnection.INPUT_CONTENT_GRANT_READ_URI_PERMISSION, null)) return
        }
        getSystemService(ClipboardManager::class.java)?.setPrimaryClip(ClipData.newUri(contentResolver, "图片", uri))
        toast("这里不能直接发图片，已放进剪贴板，长按输入框粘贴")
    }

    /** 加 / 改常用语：收起面板，键盘上方摆编辑框，打的字进编辑框；完成或收起回常用语页。 */
    override fun editPhrase(phrase: PhraseItem?, group: String) {
        closeEditor()
        closePanel()
        composer.abandon()
        val groups = ClipboardStore.phrases(this).groups.let { if (group in it) it else it + group }
        val editor = PhraseEditor(
            this, Palette.of(this), groups, phrase, group,
            onDone = { text, chosenGroup, code ->
                if (phrase == null) {
                    ClipboardStore.addPhrase(this, text, chosenGroup, code)
                } else {
                    ClipboardStore.updatePhrase(this, phrase.index, text, chosenGroup, code)
                }
                if (text.isNotBlank()) ClipboardPanel.lastGroup = chosenGroup.trim().ifEmpty { PhraseRules.defaultGroup }
                phrasesChanged()
                closeEditor()
                openPanel(PanelKind.PHRASES)
            },
            onCancel = {
                closeEditor()
                openPanel(PanelKind.PHRASES)
            },
            onFocusCode = { code ->
                composer.abandon()
                composer.directInput = code || currentInputEditorInfo?.wantsDirectInput() == true
                showKeyboard()
            },
        )
        engine.setMetered(false)
        this.editor = editor
        panel?.showEditor(editor)
    }

    override fun editClip(index: Int, text: String) {
        closeEditor()
        closePanel()
        composer.abandon()
        val editor = PhraseEditor(
            this, Palette.of(this), emptyList(), null, "",
            onDone = { edited, _, _ ->
                ClipboardStore.edit(this, index, text, edited, clipboardLimit)
                ClipboardSync.changed()
                closeEditor()
                openPanel(PanelKind.CLIPBOARD)
            },
            onCancel = {
                closeEditor()
                openPanel(PanelKind.CLIPBOARD)
            },
            onFocusCode = {},
            clip = text,
        )
        engine.setMetered(false)
        this.editor = editor
        panel?.showEditor(editor)
    }

    /**
     * 工具栏的翻译：开快捷翻译条（再点一下收起，译文留着）。没配云端就提示去设置。
     * 方向胶囊写学习语言。
     */
    private fun toggleTranslate() {
        if (editor is TranslateBar) {
            closeEditor()
            return
        }
        if (!cloud.sync(translate = true)) {
            needsSetup(getString(R.string.translate_needs_setup), CloudSettingsActivity::class.java)
            return
        }
        closeEditor()
        closePanel()
        composer.abandon()
        val bar = TranslateBar(
            this, Palette.of(this), partner = "英",
            onSource = quickTranslate::onSource,
            onConfirm = { closeEditor() },
            onCancel = {
                quickTranslate.discard()
                closeEditor()
            },
        )
        quickTranslate.bar = bar
        engine.setMetered(false)
        editor = bar
        panel?.showEditor(bar)
        showKeyboard()
    }

    private fun closeEditor() {
        if (editor == null) return
        // 快捷翻译收起（确定、再点图标、换输入框）都把已有的译文落定；取消在这之前已经撤掉了
        if (editor is TranslateBar) quickTranslate.finish()
        composer.abandon()
        // 编辑条里打的字不进应用，不算「打了多少字」；收起后照常记
        engine.setMetered(true)
        editor = null
        composer.directInput = currentInputEditorInfo?.wantsDirectInput() == true
        panel?.showEditor(null)
        showKeyboard()
    }

    /** 工具栏的话筒、全面屏底栏的语音键：点一下开始说，说完点「完成」。 */
    private fun voiceInput() {
        voice.start(holding = false)
    }

    override fun onVoiceHold(): Boolean = voice.start(holding = true)

    override fun onVoiceDrag(screenX: Float, screenY: Float) = voice.drag(screenX, screenY)

    override fun onVoiceRelease(cancel: Boolean) = voice.release(cancel)

    override fun onSwipe(input: String) = composer.onSwipe(input)

    override fun onBackspaceDown() = composer.onBackspaceDown()

    override fun onClearBefore() = composer.clearBefore()

    // 回车键写的字（发送、搜索…）照搬到面板右边的键上；多行框没有动作，写「完成」
    override fun openVoicePanel(actions: VoicePanel.Actions): VoicePanel? =
        panel?.openVoice((editor?.enterLabel ?: enterLabel) ?: getString(R.string.voice_done), actions)

    override fun hideKeyboard() = onHideKeyboard()

    override fun closeVoicePanel() {
        panel?.close()
    }

    override fun prepareVoice() = composer.prepareVoice()

    // 走 [connection]：快捷翻译、常用语编辑条开着时字进编辑条
    override fun composeVoice(text: String) {
        val connection = connection ?: return
        connection.setComposingText(text, 1)
        if (text.isEmpty()) connection.finishComposingText()
    }

    override fun commitVoice(text: String) = composer.commitVoice(text)

    override fun performVoiceAction() {
        if (editor == null && enterLabel != null) performEditorAction()
    }

    override fun onResizeDrag(deltaPx: Float, done: Boolean) {
        val keyboard = panel?.keyboard ?: return
        val from = resizeFrom ?: Settings.load(this).heightPercent.also { resizeFrom = it }
        val percent = (from + deltaPx / keyboard.baseHeight * 100).toInt()
            .coerceIn(Settings.HEIGHT_MIN, Settings.HEIGHT_MAX)
        keyboard.setHeightPercent(percent)
        if (done) {
            resizeFrom = null
            writeSettings { Settings.preferences(this).edit().putInt(Settings.KEY_HEIGHT, percent).commit() }
        }
    }
}
