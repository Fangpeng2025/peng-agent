package com.peng.agent.service

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.GestureDescription
import android.content.Intent
import android.graphics.Path
import android.graphics.Rect
import android.os.Build
import android.util.Log
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo

/**
 * 鹏Agent 无障碍服务
 *
 * 提供屏幕元素遍历、点击、滑动、输入等自动化能力。
 * 通过静态 [instance] 暴露给 [ToolExecutor] 使用。
 */
class PengAccessibilityService : AccessibilityService() {

    companion object {
        private const val TAG = "PengAccessibility"

        @Volatile
        var instance: PengAccessibilityService? = null
            private set

        fun isRunning(): Boolean = instance != null
    }

    // ── Lifecycle ──────────────────────────────────────────────────────────

    override fun onServiceConnected() {
        super.onServiceConnected()
        instance = this
        Log.i(TAG, "✅ 无障碍服务已连接")
    }

    override fun onDestroy() {
        super.onDestroy()
        instance = null
        Log.i(TAG, "⚠️ 无障碍服务已断开")
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        // 事件由 ToolExecutor 按需读取，此处不做主动处理
    }

    override fun onInterrupt() {
        Log.w(TAG, "⚠️ 无障碍服务被中断")
    }

    // ── Utility: perform gesture ───────────────────────────────────────────

    fun performTap(x: Float, y: Float, duration: Long = 100): Boolean {
        val path = Path().apply { moveTo(x, y) }
        val stroke = GestureDescription.StrokeDescription(path, 0, duration)
        val gesture = GestureDescription.Builder().addStroke(stroke).build()
        return dispatchGesture(gesture, null, null)
    }

    fun performSwipe(
        startX: Float, startY: Float,
        endX: Float, endY: Float,
        duration: Long = 300
    ): Boolean {
        val path = Path().apply {
            moveTo(startX, startY)
            lineTo(endX, endY)
        }
        val stroke = GestureDescription.StrokeDescription(path, 0, duration)
        val gesture = GestureDescription.Builder().addStroke(stroke).build()
        return dispatchGesture(gesture, null, null)
    }

    fun performLongPress(x: Float, y: Float, duration: Long = 500): Boolean {
        val path = Path().apply { moveTo(x, y) }
        val stroke = GestureDescription.StrokeDescription(path, 0, duration)
        val gesture = GestureDescription.Builder().addStroke(stroke).build()
        return dispatchGesture(gesture, null, null)
    }

    fun performDrag(
        startX: Float, startY: Float,
        endX: Float, endY: Float,
        duration: Long = 500
    ): Boolean {
        val path = Path().apply {
            moveTo(startX, startY)
            lineTo(endX, endY)
        }
        val stroke = GestureDescription.StrokeDescription(path, 0, duration)
        val gesture = GestureDescription.Builder().addStroke(stroke).build()
        return dispatchGesture(gesture, null, null)
    }

    // ── Utility: find nodes ────────────────────────────────────────────────

    fun findNodesByText(text: String): List<AccessibilityNodeInfo> {
        val root = rootInActiveWindow ?: return emptyList()
        return root.findAccessibilityNodeInfosByText(text)
    }

    fun findNodesByViewId(viewId: String): List<AccessibilityNodeInfo> {
        val root = rootInActiveWindow ?: return emptyList()
        return root.findAccessibilityNodeInfosByViewId(viewId)
    }

    fun findClickableNodeByText(text: String): AccessibilityNodeInfo? {
        val nodes = findNodesByText(text)
        for (node in nodes) {
            if (node.isClickable) return node
            val parent = node.parent
            if (parent?.isClickable == true) {
                node.recycle()
                return parent
            }
        }
        nodes.forEach { it.recycle() }
        return null
    }

    // ── Utility: screen info ───────────────────────────────────────────────

    fun getScreenBounds(): Rect? {
        val root = rootInActiveWindow ?: return null
        val bounds = Rect()
        root.getBoundsInScreen(bounds)
        root.recycle()
        return bounds
    }

    fun getCurrentPackage(): String? {
        val root = rootInActiveWindow ?: return null
        val pkg = root.packageName?.toString()
        root.recycle()
        return pkg
    }
}
