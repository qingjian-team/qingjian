//! 把物理候选窗中的词条和翻页按钮提供为可点击的无障碍节点。

package app.qingjian.inputmethod.candidates.physical

import android.graphics.Rect
import android.os.Bundle
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo
import android.view.accessibility.AccessibilityNodeProvider
import app.qingjian.inputmethod.R
import app.qingjian.inputmethod.candidates.physical.virtual.CandidateItem
import app.qingjian.inputmethod.candidates.physical.virtual.Page

internal const val ACCESSIBILITY_BUTTON_CLASS = "android.widget.Button"

internal class CandidatesNodeProvider(private val host: PhysicalCandidatesView) : AccessibilityNodeProvider() {
    private val screenLocation = IntArray(2)

    private val tempRect = Rect()

    private fun pageLabel(step: Int): String = host.context.getString(
        if (step < 0) R.string.physical_previous_page else R.string.physical_next_page,
    )

    override fun createAccessibilityNodeInfo(virtualViewId: Int): AccessibilityNodeInfo? {
        if (virtualViewId == HOST_VIEW_ID) {
            @Suppress("DEPRECATION")
            val info = AccessibilityNodeInfo.obtain(host)
            host.onInitializeAccessibilityNodeInfo(info)
            info.isFocusable = false
            info.isClickable = false
            for (item in host.virtualItems()) info.addChild(host, item.virtualId)
            return info
        }

        val item = host.virtualItems().firstOrNull { it.virtualId == virtualViewId } ?: return null
        @Suppress("DEPRECATION")
        val info = AccessibilityNodeInfo.obtain(host, virtualViewId)
        info.setParent(host)
        info.packageName = host.context.packageName
        info.className = ACCESSIBILITY_BUTTON_CLASS
        info.isEnabled = true
        info.isClickable = true
        info.isFocusable = false
        info.isAccessibilityFocused = host.focusedVirtualId == virtualViewId

        when (item) {
            is CandidateItem -> {
                info.text = item.text
                info.contentDescription = item.text
            }
            is Page -> info.contentDescription = pageLabel(item.step)
        }

        host.virtualBounds(item.rect, tempRect)
        @Suppress("DEPRECATION")
        info.setBoundsInParent(tempRect)
        info.isVisibleToUser = host.isVirtualContentShown() && !tempRect.isEmpty
        host.getLocationOnScreen(screenLocation)
        tempRect.offset(screenLocation[0], screenLocation[1])
        info.setBoundsInScreen(tempRect)

        info.addAction(AccessibilityNodeInfo.AccessibilityAction.ACTION_CLICK)
        if (host.focusedVirtualId == virtualViewId) {
            info.addAction(AccessibilityNodeInfo.AccessibilityAction.ACTION_CLEAR_ACCESSIBILITY_FOCUS)
        } else {
            info.addAction(AccessibilityNodeInfo.AccessibilityAction.ACTION_ACCESSIBILITY_FOCUS)
        }
        return info
    }

    override fun performAction(virtualViewId: Int, action: Int, arguments: Bundle?): Boolean {
        if (virtualViewId == HOST_VIEW_ID) {
            return host.performHostAccessibilityAction(action, arguments)
        }
        val item = host.virtualItems().firstOrNull { it.virtualId == virtualViewId } ?: return false
        return when (action) {
            AccessibilityNodeInfo.ACTION_CLICK -> {
                host.activateVirtualItem(item)
                host.sendVirtualEvent(virtualViewId, AccessibilityEvent.TYPE_VIEW_CLICKED)
                true
            }

            AccessibilityNodeInfo.ACTION_ACCESSIBILITY_FOCUS -> {
                if (host.focusedVirtualId == virtualViewId) return false
                host.focusedVirtualId = virtualViewId
                host.invalidate()
                host.sendVirtualEvent(virtualViewId, AccessibilityEvent.TYPE_VIEW_ACCESSIBILITY_FOCUSED)
                true
            }

            AccessibilityNodeInfo.ACTION_CLEAR_ACCESSIBILITY_FOCUS -> {
                if (host.focusedVirtualId != virtualViewId) return false
                host.focusedVirtualId = HOST_VIEW_ID
                host.invalidate()
                host.sendVirtualEvent(virtualViewId, AccessibilityEvent.TYPE_VIEW_ACCESSIBILITY_FOCUS_CLEARED)
                true
            }

            else -> false
        }
    }
}
