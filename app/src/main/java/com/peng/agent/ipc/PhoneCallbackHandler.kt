package com.peng.agent.ipc

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.util.Log
import org.json.JSONObject

/**
 * Phone 工具回调处理器
 * 处理需要调用 Android API 的工具
 */
class PhoneCallbackHandler(
    private val context: Context,
    private val daemonClient: PengDaemonClient
) {
    companion object {
        private const val TAG = "PhoneCallbackHandler"
    }
    
    /**
     * 处理 Phone 回调事件
     */
    suspend fun handleCallback(event: PengDaemonClient.StreamEvent.PhoneCallback): JSONObject {
        Log.i(TAG, "Handling phone callback: ${event.tool}")
        
        val result = when (event.tool) {
            "send_notification" -> sendNotification(event.params)
            "set_clipboard" -> setClipboard(event.params)
            "get_clipboard" -> getClipboard()
            "open_app" -> openApp(event.params)
            "open_url" -> openUrl(event.params)
            "take_screenshot" -> takeScreenshot()
            "perform_gesture" -> performGesture(event.params)
            "vibrate" -> vibrate(event.params)
            "make_toast" -> makeToast(event.params)
            else -> JSONObject().put("error", "Unknown tool: ${event.tool}")
        }
        
        // 发送回调结果给 daemon
        daemonClient.sendCallbackResult(event.callbackId, result)
        
        return result
    }
    
    /**
     * 发送通知
     */
    private fun sendNotification(params: JSONObject): JSONObject {
        return try {
            val title = params.optString("title", "PengAgent")
            val content = params.optString("content", "")
            
            // TODO: 实现通知发送
            Log.i(TAG, "Notification: $title - $content")
            
            JSONObject().put("success", true)
        } catch (e: Exception) {
            JSONObject().put("error", e.message)
        }
    }
    
    /**
     * 设置剪贴板
     */
    private fun setClipboard(params: JSONObject): JSONObject {
        return try {
            val text = params.getString("text")
            val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
            val clip = ClipData.newPlainText("PengAgent", text)
            clipboard.setPrimaryClip(clip)
            
            JSONObject().put("success", true)
        } catch (e: Exception) {
            JSONObject().put("error", e.message)
        }
    }
    
    /**
     * 获取剪贴板内容
     */
    private fun getClipboard(): JSONObject {
        return try {
            val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
            val text = clipboard.primaryClip?.getItemAt(0)?.text?.toString() ?: ""
            
            JSONObject().apply {
                put("success", true)
                put("text", text)
            }
        } catch (e: Exception) {
            JSONObject().put("error", e.message)
        }
    }
    
    /**
     * 打开应用
     */
    private fun openApp(params: JSONObject): JSONObject {
        return try {
            val packageName = params.getString("package_name")
            val intent = context.packageManager.getLaunchIntentForPackage(packageName)
            
            if (intent != null) {
                intent.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK)
                context.startActivity(intent)
                JSONObject().put("success", true)
            } else {
                JSONObject().put("error", "App not found: $packageName")
            }
        } catch (e: Exception) {
            JSONObject().put("error", e.message)
        }
    }
    
    /**
     * 打开 URL
     */
    private fun openUrl(params: JSONObject): JSONObject {
        return try {
            val url = params.getString("url")
            val intent = android.content.Intent(android.content.Intent.ACTION_VIEW)
            intent.data = android.net.Uri.parse(url)
            intent.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK)
            context.startActivity(intent)
            
            JSONObject().put("success", true)
        } catch (e: Exception) {
            JSONObject().put("error", e.message)
        }
    }
    
    /**
     * 截屏 (需要辅助服务)
     */
    private fun takeScreenshot(): JSONObject {
        // TODO: 实现截屏功能（需要无障碍服务）
        return JSONObject().put("error", "Screenshot requires accessibility service")
    }
    
    /**
     * 执行手势 (需要辅助服务)
     */
    private fun performGesture(params: JSONObject): JSONObject {
        // TODO: 实现手势功能（需要无障碍服务）
        return JSONObject().put("error", "Gesture requires accessibility service")
    }
    
    /**
     * 震动
     */
    private fun vibrate(params: JSONObject): JSONObject {
        return try {
            val duration = params.optLong("duration_ms", 100)
            val vibrator = context.getSystemService(Context.VIBRATOR_SERVICE) as android.os.Vibrator
            
            if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.O) {
                vibrator.vibrate(android.os.VibrationEffect.createOneShot(
                    duration,
                    android.os.VibrationEffect.DEFAULT_AMPLITUDE
                ))
            } else {
                @Suppress("DEPRECATION")
                vibrator.vibrate(duration)
            }
            
            JSONObject().put("success", true)
        } catch (e: Exception) {
            JSONObject().put("error", e.message)
        }
    }
    
    /**
     * 显示 Toast
     */
    private fun makeToast(params: JSONObject): JSONObject {
        return try {
            val message = params.getString("message")
            val duration = if (params.optBoolean("long", false)) {
                android.widget.Toast.LENGTH_LONG
            } else {
                android.widget.Toast.LENGTH_SHORT
            }
            
            android.widget.Toast.makeText(context, message, duration).show()
            
            JSONObject().put("success", true)
        } catch (e: Exception) {
            JSONObject().put("error", e.message)
        }
    }
}