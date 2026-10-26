package app.qingjian.inputmethod.translate

import android.app.Activity
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Intent
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.text.method.ScrollingMovementMethod
import android.view.Gravity
import android.view.View
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.cloud.CloudConfig
import app.qingjian.inputmethod.core.TranslationEngine
import app.qingjian.inputmethod.settings.page.KeyboardSettingsActivity
import app.qingjian.inputmethod.settings.widget.SettingsWidgets
import app.qingjian.inputmethod.theme.AppColors

/**
 * 系统文字选择菜单里的「青简翻译」（`ACTION_PROCESS_TEXT`）：底部半屏卡片盖在原应用上，上面原文、下面译文，
 * 「复制」放进剪贴板，原文可改时另有「替换」把译文交回去换掉选中的文字。点卡片外或返回收起。
 * 翻译走 Core 的 `request_translation` 与键盘设置里的云端配置，同快捷翻译；没配云端就提示去设置。
 */
class ProcessTextActivity : Activity() {
    private lateinit var ui: SettingsWidgets
    private lateinit var result: TextView
    private lateinit var actions: LinearLayout
    private var engine: TranslationEngine? = null
    private val handler = Handler(Looper.getMainLooper())
    private var sentAt = 0L

    /** 选中的文字；空的直接收起。 */
    private var source = ""

    /** 原应用允许改（没给 `EXTRA_PROCESS_TEXT_READONLY`）时才有「替换」。 */
    private var replaceable = false

    private var translated: String? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        source = intent.getCharSequenceExtra(Intent.EXTRA_PROCESS_TEXT)?.toString()?.trim().orEmpty()
        if (source.isEmpty()) {
            finish()
            return
        }
        replaceable = !intent.getBooleanExtra(Intent.EXTRA_PROCESS_TEXT_READONLY, false)
        ui = SettingsWidgets(this, AppColors.of(this))
        setContentView(sheet())
        if (!CloudConfig.load(this).ready) {
            showStatus(getString(R.string.process_text_no_cloud))
            showActions(Actions.SETTINGS)
            return
        }
        showStatus(getString(R.string.process_text_translating))
        engine = TranslationEngine(this) { loaded -> if (loaded) send() else showFailure(R.string.process_text_failed) }
    }

    /** 发出去并开始取译文；重试也走这里。 */
    private fun send() {
        showStatus(getString(R.string.process_text_translating))
        actions.removeAllViews()
        if (engine?.translate(CloudConfig.load(this), source) == true) {
            sentAt = SystemClock.uptimeMillis()
            handler.postDelayed(poll, POLL_MS)
        } else {
            showFailure(R.string.process_text_failed)
        }
    }

    override fun onDestroy() {
        handler.removeCallbacks(poll)
        engine?.close()
        engine = null
        super.onDestroy()
    }

    private val poll: Runnable = Runnable {
        val text = engine?.poll()
        when {
            text == null && SystemClock.uptimeMillis() - sentAt > GIVE_UP_MS -> showFailure(R.string.process_text_timeout)
            text == null -> handler.postDelayed(poll, POLL_MS)
            // 出错、解析不了回包、模型没给内容，Core 都回空串
            text.isEmpty() -> showFailure(R.string.process_text_empty)
            else -> {
                translated = text
                result.text = text
                result.setTextColor(ui.colors.text)
                result.background = null
                result.setPadding(0, 0, 0, 0)
                showActions(Actions.RESULT)
            }
        }
    }

    /** 灰底铺满（点了收起），卡片贴底；卡片里吞掉点击。 */
    private fun sheet(): View {
        val card = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            background = GradientDrawable().apply {
                setColor(ui.colors.surface)
                val r = ui.dp(24).toFloat()
                cornerRadii = floatArrayOf(r, r, r, r, 0f, 0f, 0f, 0f)
            }
            setPadding(ui.dp(22), ui.dp(20), ui.dp(22), ui.dp(16))
            isClickable = true
            setOnApplyWindowInsetsListener { view, insets ->
                view.setPadding(ui.dp(22), ui.dp(20), ui.dp(22), ui.dp(16) + insets.systemWindowInsetBottom)
                insets
            }
            addView(ui.text(getString(R.string.process_text_title), 17f, ui.colors.text, bold = true))
            addView(label(R.string.process_text_source))
            addView(scrolling(source, ui.colors.secondary, maxLines = 4))
            addView(View(context).apply { setBackgroundColor(ui.colors.divider) }, LinearLayout.LayoutParams(MATCH_PARENT, 1).apply {
                topMargin = ui.dp(14)
            })
            addView(label(R.string.process_text_result))
            result = scrolling("", ui.colors.secondary, maxLines = 10)
            addView(result)
            actions = LinearLayout(context).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.END
            }
            addView(actions, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT).apply { topMargin = ui.dp(18) })
        }
        return FrameLayout(this).apply {
            setOnClickListener { finish() }
            addView(card, FrameLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT, Gravity.BOTTOM))
        }
    }

    private fun label(text: Int) = ui.text(getString(text), 12.5f, ui.colors.secondary).apply {
        setPadding(0, ui.dp(14), 0, ui.dp(6))
    }

    private fun scrolling(text: String, color: Int, maxLines: Int) = ui.text(text, 16f, color).apply {
        this.maxLines = maxLines
        movementMethod = ScrollingMovementMethod()
        setTextIsSelectable(true)
        setLineSpacing(0f, 1.25f)
    }

    private fun showStatus(text: String) {
        result.text = text
        result.setTextColor(ui.colors.secondary)
        result.background = null
        result.setPadding(0, 0, 0, 0)
    }

    /** 失败：浅红底框里的红字，以「翻译失败」开头，不给复制 / 替换，免得当成译文用掉；给「重试」。 */
    private fun showFailure(message: Int) {
        result.text = getString(message)
        result.setTextColor(FAILURE)
        result.background = ui.rounded(FAILURE_SOFT, 12f)
        result.setPadding(ui.dp(14), ui.dp(12), ui.dp(14), ui.dp(12))
        showActions(Actions.RETRY)
    }

    private enum class Actions { RESULT, RETRY, SETTINGS }

    /** 译好了：复制 + 能改原文时的替换；失败：重试；没配云端：去设置。 */
    private fun showActions(kind: Actions) {
        actions.removeAllViews()
        when (kind) {
            Actions.SETTINGS -> {
                actions.addView(button(R.string.process_text_settings, primary = true) {
                    startActivity(Intent(this, KeyboardSettingsActivity::class.java))
                    finish()
                })
                return
            }
            Actions.RETRY -> {
                actions.addView(button(R.string.process_text_retry, primary = true) { send() })
                return
            }
            Actions.RESULT -> Unit
        }
        actions.addView(button(R.string.process_text_copy, primary = !replaceable) { copy() })
        if (replaceable) {
            actions.addView(button(R.string.process_text_replace, primary = true) { replace() }, LinearLayout.LayoutParams(WRAP_CONTENT, WRAP_CONTENT).apply {
                marginStart = ui.dp(12)
            })
        }
    }

    private fun button(text: Int, primary: Boolean, onClick: () -> Unit) =
        ui.text(getString(text), 15f, if (primary) ui.colors.onBrand else ui.colors.brand, bold = true).apply {
            gravity = Gravity.CENTER
            minWidth = ui.dp(96)
            setPadding(ui.dp(20), ui.dp(12), ui.dp(20), ui.dp(12))
            background = ui.rounded(if (primary) ui.colors.brand else ui.colors.brandSoft, 22f)
            setOnClickListener { onClick() }
        }

    private fun copy() {
        val text = translated ?: return
        getSystemService(ClipboardManager::class.java)?.setPrimaryClip(ClipData.newPlainText("青简翻译", text))
        Toast.makeText(this, R.string.process_text_copied, Toast.LENGTH_SHORT).show()
        finish()
    }

    /** 译文交回原应用，换掉选中的文字。 */
    private fun replace() {
        val text = translated ?: return
        setResult(RESULT_OK, Intent().putExtra(Intent.EXTRA_PROCESS_TEXT, text))
        finish()
    }

    private companion object {
        const val POLL_MS = 100L

        /** Rust 侧请求超时是 10 秒，到时会回空结果；这里只是兜底。 */
        const val GIVE_UP_MS = 15_000L

        /** 失败提示的红字与浅红底。 */
        const val FAILURE = 0xFFD93B3B.toInt()
        const val FAILURE_SOFT = 0x1FE5484D
    }
}
