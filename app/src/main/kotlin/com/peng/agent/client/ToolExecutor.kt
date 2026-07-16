package com.peng.agent.client

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.GestureDescription
import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.Rect
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.provider.Settings
import android.util.Base64
import android.util.Log
import android.view.accessibility.AccessibilityNodeInfo
import com.peng.agent.service.PengAccessibilityService
import com.peng.agent.service.OCRManager
import com.peng.agent.util.ToolNameCn
import org.json.JSONArray
import org.json.JSONObject
import java.io.ByteArrayOutputStream
import java.io.File

/**
 * Kotlin端工具执行器 — 处理需要在Android原生层执行的工具
 *
 * 这些工具无法在Rust后端中执行，因为它们需要Android系统API：
 * - 屏幕操作（点击、滑动、输入等）
 * - 无障碍服务
 * - OCR
 * - 应用管理
 * - 剪贴板
 * - Canvas/Video渲染
 */
object ToolExecutor {

    private const val TAG = "ToolExecutor"

    private var appContext: Context? = null

    fun setAppContext(context: Context) {
        appContext = context.applicationContext
    }

    // ── Main dispatch ──────────────────────────────────────────────────────

    fun execute(toolName: String, arguments: String): String {
        return try {
            when (toolName) {
                // 屏幕操作
                "get_screen" -> getScreen(arguments)
                "click_element" -> clickElement(arguments)
                "long_click_element" -> longClickElement(arguments)
                "double_click_element" -> doubleClickElement(arguments)
                "type_text" -> typeText(arguments)
                "scroll_screen" -> scrollScreen(arguments)
                "swipe_screen" -> swipeScreen(arguments)
                "press_key" -> pressKey(arguments)
                "launch_app" -> launchApp(arguments)
                "list_apps" -> listApps(arguments)
                "ocr_region" -> ocrRegion(arguments)

                // 剪贴板
                "copy_text" -> copyText(arguments)
                "paste_text" -> pasteText(arguments)
                "cut_text" -> cutText(arguments)
                "select_all_text" -> selectAllText(arguments)
                "clear_text" -> clearText(arguments)

                // Canvas
                "canvas_create" -> canvasCreate(arguments)
                "canvas_list" -> canvasList(arguments)
                "canvas_render" -> canvasRender(arguments)

                // Video
                "video_render" -> videoRender(arguments)
                "video_understand" -> videoUnderstand(arguments)

                else -> "⚠️ 不应在Kotlin端执行: $toolName"
            }
        } catch (e: Exception) {
            Log.e(TAG, "工具执行异常: $toolName", e)
            """{"success":false,"error":"${e.message}"}"""
        }
    }

    // ── Screen operations ──────────────────────────────────────────────────

    private fun getScreen(arguments: String): String {
        val service = PengAccessibilityService.instance
            ?: return """{"success":false,"error":"无障碍服务未启动"}"""

        return try {
            val args = if (arguments.isNotBlank()) JSONObject(arguments) else JSONObject()
            val includeScreenshot = args.optBoolean("screenshot", false)

            val rootNode = service.rootInActiveWindow
                ?: return """{"success":false,"error":"无法获取窗口根节点"}"""

            val elements = mutableListOf<JSONObject>()
            traverseNode(rootNode, elements, depth = 0)

            val result = JSONObject().apply {
                put("success", true)
                put("package", rootNode.packageName?.toString() ?: "")
                put("element_count", elements.size)
                put("elements", JSONArray(elements.take(500)))
            }

            rootNode.recycle()
            result.toString()
        } catch (e: Exception) {
            """{"success":false,"error":"${e.message}"}"""
        }
    }

    private fun traverseNode(node: AccessibilityNodeInfo, elements: MutableList<JSONObject>, depth: Int) {
        if (depth > 30) return
        try {
            val element = JSONObject().apply {
                put("id", node.hashCode())
                put("className", node.className?.toString() ?: "")
                put("text", node.text?.toString() ?: "")
                put("contentDescription", node.contentDescription?.toString() ?: "")
                put("bounds", JSONObject().apply {
                    val rect = Rect()
                    node.getBoundsInScreen(rect)
                    put("left", rect.left); put("top", rect.top)
                    put("right", rect.right); put("bottom", rect.bottom)
                }.toString())
                put("clickable", node.isClickable)
                put("scrollable", node.isScrollable)
                put("editable", node.isEditable)
                put("checkable", node.isCheckable)
                put("checked", node.isChecked)
                put("enabled", node.isEnabled)
                put("focused", node.isFocused)
                put("depth", depth)
                put("viewIdResourceName", node.viewIdResourceName?.toString() ?: "")
            }
            elements.add(element)
        } catch (_: Exception) {}

        for (i in 0 until node.childCount) {
            node.getChild(i)?.let { traverseNode(it, elements, depth + 1) }
        }
    }

    private fun clickElement(arguments: String): String {
        val service = PengAccessibilityService.instance
            ?: return """{"success":false,"error":"无障碍服务未启动"}"""

        return try {
            val args = JSONObject(arguments)
            val x = args.optDouble("x", -1.0)
            val y = args.optDouble("y", -1.0)

            if (x >= 0 && y >= 0) {
                // Click by coordinates using gesture
                val gesture = GestureDescription.Builder()
                    .addStroke(GestureDescription.StrokeDescription(
                        android.graphics.Path().apply {
                            moveTo(x.toFloat(), y.toFloat())
                        },
                        0, 100
                    ))
                    .build()
                service.dispatchGesture(gesture, null, null)
                """{"success":true,"action":"click","x":$x,"y":$y}"""
            } else {
                // Click by text or resource id
                val text = args.optString("text", "")
                val resourceId = args.optString("resource_id", "")
                val node = findNode(service, text, resourceId)
                    ?: return """{"success":false,"error":"未找到目标元素"}"""

                val clicked = node.performAction(AccessibilityNodeInfo.ACTION_CLICK)
                node.recycle()
                if (clicked) """{"success":true,"action":"click","text":"$text"}"""
                else """{"success":false,"error":"点击操作失败"}"""
            }
        } catch (e: Exception) {
            """{"success":false,"error":"${e.message}"}"""
        }
    }

    private fun longClickElement(arguments: String): String {
        val service = PengAccessibilityService.instance
            ?: return """{"success":false,"error":"无障碍服务未启动"}"""

        return try {
            val args = JSONObject(arguments)
            val x = args.optDouble("x", -1.0)
            val y = args.optDouble("y", -1.0)

            if (x >= 0 && y >= 0) {
                val gesture = GestureDescription.Builder()
                    .addStroke(GestureDescription.StrokeDescription(
                        android.graphics.Path().apply {
                            moveTo(x.toFloat(), y.toFloat())
                        },
                        0, 500
                    ))
                    .build()
                service.dispatchGesture(gesture, null, null)
                """{"success":true,"action":"long_click","x":$x,"y":$y}"""
            } else {
                val text = args.optString("text", "")
                val resourceId = args.optString("resource_id", "")
                val node = findNode(service, text, resourceId)
                    ?: return """{"success":false,"error":"未找到目标元素"}"""

                val clicked = node.performAction(AccessibilityNodeInfo.ACTION_LONG_CLICK)
                node.recycle()
                if (clicked) """{"success":true,"action":"long_click","text":"$text"}"""
                else """{"success":false,"error":"长按操作失败"}"""
            }
        } catch (e: Exception) {
            """{"success":false,"error":"${e.message}"}"""
        }
    }

    private fun doubleClickElement(arguments: String): String {
        val service = PengAccessibilityService.instance
            ?: return """{"success":false,"error":"无障碍服务未启动"}"""

        return try {
            val args = JSONObject(arguments)
            val x = args.optDouble("x", -1.0)
            val y = args.optDouble("y", -1.0)

            if (x >= 0 && y >= 0) {
                val path = android.graphics.Path().apply {
                    moveTo(x.toFloat(), y.toFloat())
                }
                val gesture = GestureDescription.Builder()
                    .addStroke(GestureDescription.StrokeDescription(path, 0, 100))
                    .addStroke(GestureDescription.StrokeDescription(path, 150, 100))
                    .build()
                service.dispatchGesture(gesture, null, null)
                """{"success":true,"action":"double_click","x":$x,"y":$y}"""
            } else {
                val text = args.optString("text", "")
                val resourceId = args.optString("resource_id", "")
                val node = findNode(service, text, resourceId)
                    ?: return """{"success":false,"error":"未找到目标元素"}"""

                node.performAction(AccessibilityNodeInfo.ACTION_CLICK)
                node.performAction(AccessibilityNodeInfo.ACTION_CLICK)
                node.recycle()
                """{"success":true,"action":"double_click","text":"$text"}"""
            }
        } catch (e: Exception) {
            """{"success":false,"error":"${e.message}"}"""
        }
    }

    private fun typeText(arguments: String): String {
        val service = PengAccessibilityService.instance
            ?: return """{"success":false,"error":"无障碍服务未启动"}"""

        return try {
            val args = JSONObject(arguments)
            val text = args.optString("text", "")
            val clearFirst = args.optBoolean("clear", false)

            if (clearFirst) {
                val focusedNode = service.rootInActiveWindow?.findFocus(AccessibilityNodeInfo.FOCUS_INPUT)
                focusedNode?.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, Bundle().apply {
                    putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, "")
                })
                focusedNode?.recycle()
            }

            // Use gesture-based typing through dispatchGesture or ACTION_SET_TEXT
            val focusedNode = service.rootInActiveWindow?.findFocus(AccessibilityNodeInfo.FOCUS_INPUT)
            if (focusedNode != null && focusedNode.isEditable) {
                val success = focusedNode.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, Bundle().apply {
                    putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, text)
                })
                focusedNode.recycle()
                if (success) """{"success":true,"action":"type","text":"$text"}"""
                else """{"success":false,"error":"输入文本失败"}"""
            } else {
                focusedNode?.recycle()
                // Fallback: use IME
                service.performGlobalAction(AccessibilityService.GLOBAL_ACTION_DISMISS_NOTIFICATION_SHADE)
                """{"success":false,"error":"未找到可输入的文本框"}"""
            }
        } catch (e: Exception) {
            """{"success":false,"error":"${e.message}"}"""
        }
    }

    private fun scrollScreen(arguments: String): String {
        val service = PengAccessibilityService.instance
            ?: return """{"success":false,"error":"无障碍服务未启动"}"""

        return try {
            val args = JSONObject(arguments)
            val direction = args.optString("direction", "down")
            val success = when (direction) {
                "up" -> performScrollGesture(service, "up")
                "down" -> performScrollGesture(service, "down")
                "left" -> performScrollGesture(service, "left")
                "right" -> performScrollGesture(service, "right")
                else -> false
            }
            if (success) """{"success":true,"action":"scroll","direction":"$direction"}"""
            else """{"success":false,"error":"滚动操作失败"}"""
        } catch (e: Exception) {
            """{"success":false,"error":"${e.message}"}"""
        }
    }

    private fun swipeScreen(arguments: String): String {
        val service = PengAccessibilityService.instance
            ?: return """{"success":false,"error":"无障碍服务未启动"}"""

        return try {
            val args = JSONObject(arguments)
            val startX = args.getDouble("start_x").toFloat()
            val startY = args.getDouble("start_y").toFloat()
            val endX = args.getDouble("end_x").toFloat()
            val endY = args.getDouble("end_y").toFloat()
            val duration = args.optLong("duration", 300)

            val path = android.graphics.Path().apply {
                moveTo(startX, startY)
                lineTo(endX, endY)
            }
            val gesture = GestureDescription.Builder()
                .addStroke(GestureDescription.StrokeDescription(path, 0, duration))
                .build()
            service.dispatchGesture(gesture, null, null)
            """{"success":true,"action":"swipe","start_x":$startX,"start_y":$startY,"end_x":$endX,"end_y":$endY}"""
        } catch (e: Exception) {
            """{"success":false,"error":"${e.message}"}"""
        }
    }

    private fun pressKey(arguments: String): String {
        val service = PengAccessibilityService.instance
            ?: return """{"success":false,"error":"无障碍服务未启动"}"""

        return try {
            val args = JSONObject(arguments)
            val key = args.optString("key", "back")
            val success = when (key) {
                "back" -> service.performGlobalAction(AccessibilityService.GLOBAL_ACTION_BACK)
                "home" -> service.performGlobalAction(AccessibilityService.GLOBAL_ACTION_HOME)
                "recents" -> service.performGlobalAction(AccessibilityService.GLOBAL_ACTION_RECENTS)
                "notifications" -> service.performGlobalAction(AccessibilityService.GLOBAL_ACTION_NOTIFICATIONS)
                "quick_settings" -> service.performGlobalAction(AccessibilityService.GLOBAL_ACTION_QUICK_SETTINGS)
                "power_dialog" -> service.performGlobalAction(AccessibilityService.GLOBAL_ACTION_POWER_DIALOG)
                "lock_screen" -> if (Build.VERSION.SDK_INT >= 28) service.performGlobalAction(AccessibilityService.GLOBAL_ACTION_LOCK_SCREEN) else false
                "take_screenshot" -> if (Build.VERSION.SDK_INT >= 28) service.performGlobalAction(AccessibilityService.GLOBAL_ACTION_TAKE_SCREENSHOT) else false
                else -> false
            }
            if (success) """{"success":true,"action":"press_key","key":"$key"}"""
            else """{"success":false,"error":"按键操作失败: $key"}"""
        } catch (e: Exception) {
            """{"success":false,"error":"${e.message}"}"""
        }
    }

    private fun launchApp(arguments: String): String {
        val context = appContext
            ?: return """{"success":false,"error":"应用上下文未初始化"}"""

        return try {
            val args = JSONObject(arguments)
            val packageName = args.optString("package", "")
            if (packageName.isEmpty()) return """{"success":false,"error":"未指定包名"}"""

            val pm = context.packageManager
            val intent = pm.getLaunchIntentForPackage(packageName)
                ?: return """{"success":false,"error":"未找到应用: $packageName"}"""

            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP)
            context.startActivity(intent)
            """{"success":true,"action":"launch","package":"$packageName"}"""
        } catch (e: Exception) {
            """{"success":false,"error":"${e.message}"}"""
        }
    }

    private fun listApps(arguments: String): String {
        val context = appContext
            ?: return """{"success":false,"error":"应用上下文未初始化"}"""

        return try {
            val pm = context.packageManager
            val apps = pm.getInstalledApplications(0)
                .filter { it.flags and android.content.pm.ApplicationInfo.FLAG_SYSTEM == 0 }
                .map { appInfo ->
                    JSONObject().apply {
                        put("package", appInfo.packageName)
                        put("name", pm.getApplicationLabel(appInfo).toString())
                    }
                }
            JSONObject().apply {
                put("success", true)
                put("count", apps.size)
                put("apps", JSONArray(apps))
            }.toString()
        } catch (e: Exception) {
            """{"success":false,"error":"${e.message}"}"""
        }
    }

    private fun ocrRegion(arguments: String): String {
        return try {
            val args = JSONObject(arguments)
            val region = args.optString("region", "")
            OCRManager.recognize(region)
        } catch (e: Exception) {
            """{"success":false,"error":"${e.message}"}"""
        }
    }

    // ── Clipboard operations ───────────────────────────────────────────────

    private fun copyText(arguments: String): String {
        val service = PengAccessibilityService.instance
            ?: return """{"success":false,"error":"无障碍服务未启动"}"""

        return try {
            val args = JSONObject(arguments)
            val text = args.optString("text", "")
            if (text.isEmpty()) {
                // Copy selected text
                val focusedNode = service.rootInActiveWindow?.findFocus(AccessibilityNodeInfo.FOCUS_INPUT)
                val copied = focusedNode?.performAction(AccessibilityNodeInfo.ACTION_COPY)
                focusedNode?.recycle()
                if (copied == true) """{"success":true,"action":"copy"}"""
                else """{"success":false,"error":"复制失败"}"""
            } else {
                // Copy specific text to clipboard
                val clipboard = service.getSystemService(Context.CLIPBOARD_SERVICE) as android.content.ClipboardManager
                clipboard.setPrimaryClip(android.content.ClipData.newPlainText("text", text))
                """{"success":true,"action":"copy","text":"$text"}"""
            }
        } catch (e: Exception) {
            """{"success":false,"error":"${e.message}"}"""
        }
    }

    private fun pasteText(arguments: String): String {
        val service = PengAccessibilityService.instance
            ?: return """{"success":false,"error":"无障碍服务未启动"}"""

        return try {
            val focusedNode = service.rootInActiveWindow?.findFocus(AccessibilityNodeInfo.FOCUS_INPUT)
            val pasted = focusedNode?.performAction(AccessibilityNodeInfo.ACTION_PASTE)
            focusedNode?.recycle()
            if (pasted == true) """{"success":true,"action":"paste"}"""
            else """{"success":false,"error":"粘贴失败"}"""
        } catch (e: Exception) {
            """{"success":false,"error":"${e.message}"}"""
        }
    }

    private fun cutText(arguments: String): String {
        val service = PengAccessibilityService.instance
            ?: return """{"success":false,"error":"无障碍服务未启动"}"""

        return try {
            val focusedNode = service.rootInActiveWindow?.findFocus(AccessibilityNodeInfo.FOCUS_INPUT)
            val cut = focusedNode?.performAction(AccessibilityNodeInfo.ACTION_CUT)
            focusedNode?.recycle()
            if (cut == true) """{"success":true,"action":"cut"}"""
            else """{"success":false,"error":"剪切失败"}"""
        } catch (e: Exception) {
            """{"success":false,"error":"${e.message}"}"""
        }
    }

    private fun selectAllText(arguments: String): String {
        val service = PengAccessibilityService.instance
            ?: return """{"success":false,"error":"无障碍服务未启动"}"""

        return try {
            val focusedNode = service.rootInActiveWindow?.findFocus(AccessibilityNodeInfo.FOCUS_INPUT)
            val selected = focusedNode?.performAction(0x40) // ACTION_SELECT_ALL
            focusedNode?.recycle()
            if (selected == true) """{"success":true,"action":"select_all"}"""
            else """{"success":false,"error":"全选失败"}"""
        } catch (e: Exception) {
            """{"success":false,"error":"${e.message}"}"""
        }
    }

    private fun clearText(arguments: String): String {
        val service = PengAccessibilityService.instance
            ?: return """{"success":false,"error":"无障碍服务未启动"}"""

        return try {
            val focusedNode = service.rootInActiveWindow?.findFocus(AccessibilityNodeInfo.FOCUS_INPUT)
            val cleared = focusedNode?.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, Bundle().apply {
                putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, "")
            })
            focusedNode?.recycle()
            if (cleared == true) """{"success":true,"action":"clear"}"""
            else """{"success":false,"error":"清除文本失败"}"""
        } catch (e: Exception) {
            """{"success":false,"error":"${e.message}"}"""
        }
    }

    // ── Canvas operations ──────────────────────────────────────────────────

    private fun canvasCreate(arguments: String): String {
        return try {
            val args = JSONObject(arguments)
            val name = args.optString("name", "canvas_${System.currentTimeMillis()}")
            val width = args.optInt("width", 1080)
            val height = args.optInt("height", 1920)

            val canvasDir = File("/sdcard/peng-agent/canvas")
            canvasDir.mkdirs()
            val canvasFile = File(canvasDir, "$name.json")
            val meta = JSONObject().apply {
                put("name", name)
                put("width", width)
                put("height", height)
                put("created", System.currentTimeMillis())
                put("elements", JSONArray())
            }
            canvasFile.writeText(meta.toString())
            """{"success":true,"name":"$name","width":$width,"height":$height}"""
        } catch (e: Exception) {
            """{"success":false,"error":"${e.message}"}"""
        }
    }

    private fun canvasList(arguments: String): String {
        return try {
            val canvasDir = File("/sdcard/peng-agent/canvas")
            if (!canvasDir.exists()) return """{"success":true,"canvases":[]}"""

            val canvases = canvasDir.listFiles()
                ?.filter { it.name.endsWith(".json") }
                ?.map { it.nameWithoutExtension }
                ?: emptyList()

            JSONObject().apply {
                put("success", true)
                put("canvases", JSONArray(canvases))
            }.toString()
        } catch (e: Exception) {
            """{"success":false,"error":"${e.message}"}"""
        }
    }

    private fun canvasRender(arguments: String): String {
        return try {
            val args = JSONObject(arguments)
            val name = args.optString("name", "")
            val elements = args.optJSONArray("elements") ?: JSONArray()

            val canvasDir = File("/sdcard/peng-agent/canvas")
            canvasDir.mkdirs()
            val canvasFile = File(canvasDir, "$name.json")

            val meta = if (canvasFile.exists()) JSONObject(canvasFile.readText()) else JSONObject().apply {
                put("name", name)
                put("width", 1080)
                put("height", 1920)
            }
            meta.put("elements", elements)
            meta.put("updated", System.currentTimeMillis())
            canvasFile.writeText(meta.toString())

            """{"success":true,"name":"$name","element_count":${elements.length()}}"""
        } catch (e: Exception) {
            """{"success":false,"error":"${e.message}"}"""
        }
    }

    // ── Video operations ───────────────────────────────────────────────────

    private fun videoRender(arguments: String): String {
        return try {
            val args = JSONObject(arguments)
            val mode = args.optString("mode", "native")
            val outputPath = args.optString("output", "/sdcard/peng-agent/video/output.mp4")

            when (mode) {
                "hyperframes" -> {
                    com.peng.agent.video.TermuxHyperFramesRenderer.render(args)
                }
                else -> {
                    // Native mode - delegate to Rust backend
                    "⚠️ 不应在Kotlin端执行: video_render (native mode)"
                }
            }
        } catch (e: Exception) {
            """{"success":false,"error":"${e.message}"}"""
        }
    }

    private fun videoUnderstand(arguments: String): String {
        // This is handled by the Rust backend with vision model
        return "⚠️ 不应在Kotlin端执行: video_understand"
    }

    // ── Helper: scroll gesture ──────────────────────────────────────────────

    private fun performScrollGesture(service: PengAccessibilityService, direction: String): Boolean {
        val rootNode = service.rootInActiveWindow ?: return false
        val width = rootNode.extras?.getInt("width") ?: 1080
        val height = rootNode.extras?.getInt("height") ?: 1920
        rootNode.recycle()

        val centerX = width / 2f
        val centerY = height / 2f
        val scrollDistance = height * 0.3f

        val (startX, startY, endX, endY) = when (direction) {
            "up" -> listOf(centerX, centerY + scrollDistance / 2, centerX, centerY - scrollDistance / 2)
            "down" -> listOf(centerX, centerY - scrollDistance / 2, centerX, centerY + scrollDistance / 2)
            "left" -> listOf(centerX + scrollDistance / 2, centerY, centerX - scrollDistance / 2, centerY)
            "right" -> listOf(centerX - scrollDistance / 2, centerY, centerX + scrollDistance / 2, centerY)
            else -> return false
        }

        val path = android.graphics.Path().apply {
            moveTo(startX, startY)
            lineTo(endX, endY)
        }
        val gesture = GestureDescription.Builder()
            .addStroke(GestureDescription.StrokeDescription(path, 0, 300))
            .build()
        return service.dispatchGesture(gesture, null, null)
    }

    // ── Helper: find accessibility node ────────────────────────────────────

    private fun findNode(
        service: PengAccessibilityService,
        text: String,
        resourceId: String
    ): AccessibilityNodeInfo? {
        val rootNode = service.rootInActiveWindow ?: return null

        if (text.isNotEmpty()) {
            val nodes = rootNode.findAccessibilityNodeInfosByText(text)
            for (node in nodes) {
                if (node.isClickable || node.parent?.isClickable == true) {
                    rootNode.recycle()
                    return if (node.isClickable) node else node.parent.also { node.recycle() }
                }
            }
            nodes.forEach { it.recycle() }
        }

        if (resourceId.isNotEmpty()) {
            val nodes = rootNode.findAccessibilityNodeInfosByViewId(resourceId)
            for (node in nodes) {
                rootNode.recycle()
                return node
            }
            nodes.forEach { it.recycle() }
        }

        rootNode.recycle()
        return null
    }
}
