//! 物理键盘候选视图：宿主桌面竖排渲染位图，处理自适应缩放、共享位图隔离、安全点击与无障碍节点。

package app.qingjian.inputmethod.candidates.physical

import android.annotation.SuppressLint
import android.content.Context
import android.content.res.Configuration
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Rect
import android.graphics.RectF
import android.os.Bundle
import android.text.TextPaint
import android.view.MotionEvent
import android.view.View
import android.view.ViewConfiguration
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityManager
import android.view.accessibility.AccessibilityNodeProvider
import app.qingjian.inputmethod.candidates.physical.touch.CandidateItem as TouchCandidateItem
import app.qingjian.inputmethod.candidates.physical.touch.Page as TouchPage
import app.qingjian.inputmethod.candidates.physical.touch.TouchTarget
import app.qingjian.inputmethod.candidates.physical.virtual.CandidateItem as VirtualCandidateItem
import app.qingjian.inputmethod.candidates.physical.virtual.Page as VirtualPage
import app.qingjian.inputmethod.candidates.physical.virtual.VirtualItem
import app.qingjian.inputmethod.core.frame.Candidate
import app.qingjian.inputmethod.core.strip.StripImage
import kotlin.math.ceil
import kotlin.math.hypot
import kotlin.math.roundToInt

private const val VIRTUAL_PAGE_PREV = 1
private const val VIRTUAL_PAGE_NEXT = 2
private const val VIRTUAL_CANDIDATE_OFFSET = 100

/** 系统 candidates view：贴桌面渲染器位图，保留主题的序号、高亮、emoji 补绘与翻页点击。 */
@SuppressLint("ViewConstructor")
class PhysicalCandidatesView(
    context: Context,

    private val source: () -> StripImage?,

    private val select: (Int) -> Unit,

    private val page: (Int) -> Unit,
) : View(context) {
    private var snapshot: Snapshot? = null

    private var candidates: List<Candidate> = emptyList()

    private val bitmapCopy = BitmapCopy()

    private var scale = 1f

    private var drawLeft = 0f

    private var drawTop = 0f

    private val drawnRect = RectF()

    private val bitmapPaint = Paint(Paint.ANTI_ALIAS_FLAG or Paint.FILTER_BITMAP_FLAG)

    private val emojiPaint = TextPaint(Paint.ANTI_ALIAS_FLAG).apply { textAlign = Paint.Align.CENTER }

    private val touchSlop = ViewConfiguration.get(context).scaledTouchSlop.toFloat()

    private var frameGeneration = 0L

    private var downGeneration = 0L

    private var activePointerId = MotionEvent.INVALID_POINTER_ID

    private var downX = 0f

    private var downY = 0f

    private var downTarget: TouchTarget? = null

    private var gestureCancelled = false

    private val accessibilityManager = context.getSystemService(AccessibilityManager::class.java)

    private var hoveredVirtualId = AccessibilityNodeProvider.HOST_VIEW_ID

    internal var focusedVirtualId = AccessibilityNodeProvider.HOST_VIEW_ID

    private val nodeProvider = CandidatesNodeProvider(this)

    private val refreshOnConfig = Runnable {
        if (isAttachedToWindow && (candidates.isNotEmpty() || snapshot != null)) refresh(candidates, contentDescription?.toString())
    }

    init {
        isFocusable = false
        isFocusableInTouchMode = false
    }

    fun refresh(items: List<Candidate>, modeLabel: String? = null) {
        contentDescription = modeLabel
        removeCallbacks(refreshOnConfig)
        candidates = items
        frameGeneration++
        cancelTouch()
        hoveredVirtualId = AccessibilityNodeProvider.HOST_VIEW_ID
        focusedVirtualId = AccessibilityNodeProvider.HOST_VIEW_ID

        val rendered = source()
        val hasPreedit = rendered?.let(::hasPreedit) == true
        val copied = rendered?.takeIf { items.isNotEmpty() || hasPreedit || modeLabel != null }?.let { bitmapCopy.copy(it.bitmap) }
        snapshot = if (rendered != null && copied != null) {
            Snapshot(
                bitmap = copied,
                width = copied.width,
                height = copied.height,
                background = rendered.background,
                hits = rendered.hits,
                pages = rendered.pages,
                emoji = rendered.emoji,
                rawImage = rendered,
            )
        } else {
            null
        }

        if ((width > 0 && height > 0) || snapshot == null) updateDrawBounds(width, height)
        requestLayout()
        invalidate()
        notifySubtreeChanged()
    }

    override fun onAttachedToWindow() {
        super.onAttachedToWindow()
        if (snapshot != null || candidates.isNotEmpty()) {
            requestLayout()
            invalidate()
        }
    }

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val padH = paddingLeft + paddingRight
        val padV = paddingTop + paddingBottom
        val snap = snapshot
        if (snap == null || snap.width <= 0 || snap.height <= 0) {
            val desiredW = maxOf(suggestedMinimumWidth, padH)
            val desiredH = maxOf(suggestedMinimumHeight, padV)
            val measuredW = resolveSizeAndState(desiredW, widthMeasureSpec, 0)
            val measuredH = resolveSizeAndState(desiredH, heightMeasureSpec, 0)
            setMeasuredDimension(measuredW, measuredH)
            return
        }

        val widthMode = MeasureSpec.getMode(widthMeasureSpec)
        val widthSize = MeasureSpec.getSize(widthMeasureSpec)
        val heightMode = MeasureSpec.getMode(heightMeasureSpec)
        val heightSize = MeasureSpec.getSize(heightMeasureSpec)

        val availW = if (widthMode == MeasureSpec.UNSPECIFIED) Int.MAX_VALUE else (widthSize - padH).coerceAtLeast(0)
        val availH = if (heightMode == MeasureSpec.UNSPECIFIED) Int.MAX_VALUE else (heightSize - padV).coerceAtLeast(0)

        // 父容器在探测测量里可能传入 0 像素 AT_MOST；只在正有限空间内等比缩小，避免把固有尺寸塌陷为 0。
        val sx = if (widthMode == MeasureSpec.UNSPECIFIED || availW <= 0 || snap.width <= availW) {
            1f
        } else {
            availW.toFloat() / snap.width
        }
        val sy = if (heightMode == MeasureSpec.UNSPECIFIED || availH <= 0 || snap.height <= availH) {
            1f
        } else {
            availH.toFloat() / snap.height
        }
        val fitScale = minOf(1f, sx, sy)

        val desiredW = maxOf(suggestedMinimumWidth, ceil(snap.width * fitScale).toInt() + padH)
        val desiredH = maxOf(suggestedMinimumHeight, ceil(snap.height * fitScale).toInt() + padV)
        // 屏幕键盘隐藏时，InputMethodService 可能用 AT_MOST=0 探测候选区。
        // 这次探测按未指定尺寸处理，否则视图会一直是 0 像素，显示候选时也无法展开。
        val measuredW = resolveCandidateSize(desiredW, widthMode, widthSize, widthMeasureSpec)
        val measuredH = resolveCandidateSize(desiredH, heightMode, heightSize, heightMeasureSpec)
        setMeasuredDimension(measuredW, measuredH)
    }

    private fun resolveCandidateSize(desired: Int, mode: Int, size: Int, spec: Int): Int =
        if (mode == MeasureSpec.AT_MOST && size == 0 && desired > 0) desired
        else resolveSizeAndState(desired, spec, 0)

    /** 内容矩形供宿主贴近光标定位及收触摸，透明阴影不算候选卡片。 */
    fun contentBounds(out: Rect) {
        val snap = snapshot ?: run { out.setEmpty(); return }
        val fit = minOf(1f, (measuredWidth - paddingLeft - paddingRight).coerceAtLeast(0).toFloat() / snap.width,
            (measuredHeight - paddingTop - paddingBottom).coerceAtLeast(0).toFloat() / snap.height)
        val rect = snap.rawImage.content
        out.set((paddingLeft + rect.left * fit).roundToInt(), (paddingTop + rect.top * fit).roundToInt(),
            (paddingLeft + rect.right * fit).roundToInt(), (paddingTop + rect.bottom * fit).roundToInt())
    }

    override fun onLayout(changed: Boolean, left: Int, top: Int, right: Int, bottom: Int) {
        super.onLayout(changed, left, top, right, bottom)
        updateDrawBounds(right - left, bottom - top)
        if (changed) notifySubtreeChanged()
    }

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
        super.onSizeChanged(w, h, oldw, oldh)
        updateDrawBounds(w, h)
    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        if (candidates.isNotEmpty() || snapshot != null) {
            removeCallbacks(refreshOnConfig)
            post(refreshOnConfig)
        }
    }

    override fun onDetachedFromWindow() {
        removeCallbacks(refreshOnConfig)
        cancelTouch()
        hoveredVirtualId = AccessibilityNodeProvider.HOST_VIEW_ID
        focusedVirtualId = AccessibilityNodeProvider.HOST_VIEW_ID
        super.onDetachedFromWindow()
    }

    override fun onDraw(canvas: Canvas) {
        val snap = snapshot ?: return
        if (snap.bitmap.isRecycled || scale <= 0f || drawnRect.isEmpty) {
            return
        }

        val save = canvas.save()
        if (paddingLeft != 0 || paddingTop != 0 || paddingRight != 0 || paddingBottom != 0) {
            canvas.clipRect(paddingLeft, paddingTop, width - paddingRight, height - paddingBottom)
        }
        canvas.translate(drawLeft, drawTop)
        canvas.scale(scale, scale)
        canvas.drawBitmap(snap.bitmap, 0f, 0f, bitmapPaint)

        // Android 彩色 emoji 沿用候选条的补绘方式；横排 row 主题的译文在右侧时会压扁 wordRect，退回整行高度。
        val dark = isDarkColor(snap.background)
        for (index in snap.emoji) {
            val cell = snap.hits.firstOrNull { it.candidate == index && it.sense == null }?.rect ?: continue
            val word = snap.wordRect(index) ?: cell
            val box = if (word.height() >= cell.height() * 0.4f) word else cell
            val text = candidates.getOrNull(index)?.text ?: continue
            var textSize = (box.height() * 0.65f).coerceAtLeast(1f)
            emojiPaint.textSize = textSize
            val measured = emojiPaint.measureText(text)
            val maxW = (box.width() * 0.9f).coerceAtLeast(1f)
            if (measured > maxW) {
                textSize = (textSize * maxW / measured).coerceAtLeast(1f)
                emojiPaint.textSize = textSize
            }
            emojiPaint.color = if (dark) Color.WHITE else Color.BLACK
            val baseline = box.centerY() - (emojiPaint.ascent() + emojiPaint.descent()) / 2f
            canvas.drawText(text, box.centerX(), baseline, emojiPaint)
        }
        canvas.restoreToCount(save)
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (!isEnabled) return false
        val snap = snapshot ?: return false
        if (scale <= 0f || drawnRect.isEmpty) return false

        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                val x = event.x
                val y = event.y
                if (!drawnRect.contains(x, y)) return false
                activePointerId = event.getPointerId(0)
                downX = x
                downY = y
                downTarget = hitTest(snap, x, y)
                downGeneration = frameGeneration
                gestureCancelled = false
                return true
            }

            MotionEvent.ACTION_POINTER_DOWN -> {
                gestureCancelled = true
                downTarget = null
                return true
            }

            MotionEvent.ACTION_MOVE -> {
                if (gestureCancelled || activePointerId == MotionEvent.INVALID_POINTER_ID) return true
                val index = event.findPointerIndex(activePointerId)
                if (index < 0) {
                    cancelTouch()
                    return true
                }
                val dx = event.getX(index) - downX
                val dy = event.getY(index) - downY
                if (hypot(dx, dy) > touchSlop) {
                    gestureCancelled = true
                    downTarget = null
                }
                return true
            }

            MotionEvent.ACTION_UP -> {
                val valid = !gestureCancelled && downGeneration == frameGeneration &&
                    activePointerId != MotionEvent.INVALID_POINTER_ID
                val target = downTarget
                val index = if (valid) event.findPointerIndex(activePointerId) else -1
                if (valid && target != null && index >= 0) {
                    val x = event.getX(index)
                    val y = event.getY(index)
                    if (hypot(x - downX, y - downY) <= touchSlop && hitTest(snap, x, y) == target) {
                        cancelTouch()
                        triggerTarget(target)
                        performClick()
                        return true
                    }
                }
                cancelTouch()
                return true
            }

            MotionEvent.ACTION_POINTER_UP -> {
                val pointerId = event.getPointerId(event.actionIndex)
                if (pointerId == activePointerId) {
                    gestureCancelled = true
                    downTarget = null
                }
                return true
            }

            MotionEvent.ACTION_CANCEL -> {
                cancelTouch()
                return true
            }
        }
        return super.onTouchEvent(event)
    }

    override fun performClick(): Boolean {
        super.performClick()
        return true
    }

    override fun dispatchHoverEvent(event: MotionEvent): Boolean {
        if (accessibilityManager?.isTouchExplorationEnabled == true) {
            when (event.actionMasked) {
                MotionEvent.ACTION_HOVER_ENTER, MotionEvent.ACTION_HOVER_MOVE -> {
                    val virtualId = virtualItemAt(event.x, event.y)?.virtualId ?: AccessibilityNodeProvider.HOST_VIEW_ID
                    updateHoveredVirtualId(virtualId)
                    return virtualId != AccessibilityNodeProvider.HOST_VIEW_ID
                }

                MotionEvent.ACTION_HOVER_EXIT -> {
                    updateHoveredVirtualId(AccessibilityNodeProvider.HOST_VIEW_ID)
                    return true
                }
            }
        }
        return super.dispatchHoverEvent(event)
    }

    override fun getAccessibilityNodeProvider(): AccessibilityNodeProvider = nodeProvider

    private fun updateDrawBounds(viewWidth: Int, viewHeight: Int) {
        val snap = snapshot
        if (snap == null || snap.width <= 0 || snap.height <= 0) {
            scale = 1f
            drawLeft = paddingLeft.toFloat()
            drawTop = paddingTop.toFloat()
            drawnRect.setEmpty()
            return
        }
        val availW = (viewWidth - paddingLeft - paddingRight).coerceAtLeast(0)
        val availH = (viewHeight - paddingTop - paddingBottom).coerceAtLeast(0)
        if (availW == 0 || availH == 0) {
            scale = 0f
            drawLeft = paddingLeft.toFloat()
            drawTop = paddingTop.toFloat()
            drawnRect.setEmpty()
            return
        }
        val sx = if (snap.width <= availW) 1f else availW.toFloat() / snap.width
        val sy = if (snap.height <= availH) 1f else availH.toFloat() / snap.height
        scale = minOf(1f, sx, sy)
        drawLeft = paddingLeft.toFloat()
        drawTop = paddingTop.toFloat()
        drawnRect.set(drawLeft, drawTop, drawLeft + snap.width * scale, drawTop + snap.height * scale)
    }

    private fun hitTest(snap: Snapshot, viewX: Float, viewY: Float): TouchTarget? {
        if (scale <= 0f || !drawnRect.contains(viewX, viewY)) return null
        val bx = (viewX - drawLeft) / scale
        val by = (viewY - drawTop) / scale
        snap.pages.firstOrNull { it.rect.contains(bx, by) }?.let { return TouchPage(it.step) }
        snap.hits.firstOrNull { it.sense == null && it.rect.contains(bx, by) }?.let {
            return TouchCandidateItem(it.candidate)
        }
        return null
    }

    private fun triggerTarget(target: TouchTarget) {
        when (target) {
            is TouchPage -> page(target.step)
            is TouchCandidateItem -> select(target.index)
        }
    }

    private fun cancelTouch() {
        activePointerId = MotionEvent.INVALID_POINTER_ID
        downTarget = null
        gestureCancelled = false
    }

    private fun isDarkColor(argb: Int): Boolean {
        if (Color.alpha(argb) == 0) return false
        val lum = 0.2126f * (Color.red(argb) / 255f) +
            0.7152f * (Color.green(argb) / 255f) +
            0.0722f * (Color.blue(argb) / 255f)
        return lum < 0.4f
    }

    internal fun virtualItems(): List<VirtualItem> {
        val snap = snapshot ?: return emptyList()
        val items = ArrayList<VirtualItem>(snap.hits.size + snap.pages.size)
        for (hit in snap.hits) {
            if (hit.sense != null) continue
            val text = candidates.getOrNull(hit.candidate)?.text ?: continue
            items += VirtualCandidateItem(
                virtualId = VIRTUAL_CANDIDATE_OFFSET + hit.candidate,
                index = hit.candidate,
                text = text,
                rect = hit.rect,
            )
        }
        for (p in snap.pages) {
            val id = if (p.step < 0) VIRTUAL_PAGE_PREV else VIRTUAL_PAGE_NEXT
            items += VirtualPage(id, p.step, p.rect)
        }
        return items
    }

    private fun virtualItemAt(viewX: Float, viewY: Float): VirtualItem? {
        if (scale <= 0f || !drawnRect.contains(viewX, viewY)) return null
        val bx = (viewX - drawLeft) / scale
        val by = (viewY - drawTop) / scale
        val items = virtualItems()
        items.firstOrNull { it is VirtualPage && it.rect.contains(bx, by) }?.let { return it }
        return items.firstOrNull { it is VirtualCandidateItem && it.rect.contains(bx, by) }
    }

    private fun updateHoveredVirtualId(virtualId: Int) {
        if (hoveredVirtualId == virtualId) return
        val prev = hoveredVirtualId
        hoveredVirtualId = virtualId
        if (virtualId != AccessibilityNodeProvider.HOST_VIEW_ID) {
            sendVirtualEvent(virtualId, AccessibilityEvent.TYPE_VIEW_HOVER_ENTER)
        }
        if (prev != AccessibilityNodeProvider.HOST_VIEW_ID) {
            sendVirtualEvent(prev, AccessibilityEvent.TYPE_VIEW_HOVER_EXIT)
        }
    }

    private fun notifySubtreeChanged() {
        if (accessibilityManager?.isEnabled != true || !isAttachedToWindow) return
        @Suppress("DEPRECATION")
        val event = AccessibilityEvent.obtain(AccessibilityEvent.TYPE_WINDOW_CONTENT_CHANGED)
        event.setSource(this)
        event.packageName = context.packageName
        event.className = javaClass.name
        event.contentChangeTypes = AccessibilityEvent.CONTENT_CHANGE_TYPE_SUBTREE
        parent?.requestSendAccessibilityEvent(this, event)
    }

    internal fun sendVirtualEvent(virtualId: Int, eventType: Int) {
        if (accessibilityManager?.isEnabled != true || !isAttachedToWindow) return
        @Suppress("DEPRECATION")
        val event = AccessibilityEvent.obtain(eventType)
        event.packageName = context.packageName
        event.className = ACCESSIBILITY_BUTTON_CLASS
        event.isEnabled = true
        event.setSource(this, virtualId)
        parent?.requestSendAccessibilityEvent(this, event)
    }

    private fun hasPreedit(image: StripImage): Boolean =
        image.preeditEdges.isNotEmpty() || image.preeditBottom > image.preeditTop

    internal fun virtualBounds(rect: RectF, out: Rect) {
        out.set(
            (drawLeft + rect.left * scale).roundToInt(),
            (drawTop + rect.top * scale).roundToInt(),
            (drawLeft + rect.right * scale).roundToInt(),
            (drawTop + rect.bottom * scale).roundToInt(),
        )
    }

    internal fun isVirtualContentShown(): Boolean = isShown && scale > 0f

    internal fun activateVirtualItem(item: VirtualItem) {
        when (item) {
            is VirtualCandidateItem -> select(item.index)
            is VirtualPage -> page(item.step)
        }
    }

    internal fun performHostAccessibilityAction(action: Int, arguments: Bundle?): Boolean =
        super.performAccessibilityAction(action, arguments)
}
