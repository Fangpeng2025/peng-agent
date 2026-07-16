package com.peng.agent.service

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.Build
import android.util.Log
import androidx.core.app.NotificationCompat
import com.peng.agent.MainActivity
import com.peng.agent.R

/**
 * Live Update 灵动岛管理器 — API 36+ 专用
 *
 * 在支持 Live Update 的设备上，将任务进度推送到灵动岛区域。
 * 降级方案：使用常规前台通知。
 */
object LiveUpdateIslandManager {

    private const val TAG = "LiveUpdateIsland"
    private const val CHANNEL_ID = "peng_agent_live_update"
    private const val NOTIFICATION_ID = 2002

    private var isLiveUpdateActive = false

    // ── Public API ─────────────────────────────────────────────────────────

    fun isLiveUpdateSupported(context: Context): Boolean {
        if (Build.VERSION.SDK_INT < 36) return false
        val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        return nm.canPostPromotedNotifications()
    }

    fun startLiveUpdate(context: Context, taskId: String, taskName: String) {
        if (!isLiveUpdateSupported(context)) {
            Log.w(TAG, "设备不支持 Live Update")
            return
        }

        ensureChannel(context)

        val notification = buildLiveUpdateNotification(context, taskName, "思考中...")
        val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        nm.notify(NOTIFICATION_ID, notification)
        isLiveUpdateActive = true

        Log.i(TAG, "🏝️ Live Update 已启动: $taskName")
    }

    fun updateLiveUpdate(context: Context, phase: String, toolName: String, progress: Int) {
        if (!isLiveUpdateActive) return

        val content = when (phase) {
            "thinking" -> "思考中..."
            "tool_call" -> "🔧 $toolName"
            "streaming" -> "回复中..."
            else -> "处理中..."
        }

        val state = DynamicIslandState.state
        val taskName = state.value.taskName

        val notification = buildLiveUpdateNotification(context, taskName, content, progress)
        val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        nm.notify(NOTIFICATION_ID, notification)
    }

    fun stopLiveUpdate(context: Context) {
        if (!isLiveUpdateActive) return
        val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        nm.cancel(NOTIFICATION_ID)
        isLiveUpdateActive = false
        Log.i(TAG, "🏝️ Live Update 已停止")
    }

    // ── Internal ───────────────────────────────────────────────────────────

    private fun buildLiveUpdateNotification(
        context: Context,
        title: String,
        content: String,
        progress: Int = 0
    ): Notification {
        val intent = Intent(context, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP
        }
        val pendingIntent = PendingIntent.getActivity(
            context, 0, intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        return NotificationCompat.Builder(context, CHANNEL_ID)
            .setSmallIcon(R.mipmap.ic_launcher)
            .setContentTitle(title)
            .setContentText(content)
            .setOngoing(true)
            .setSilent(true)
            .setContentIntent(pendingIntent)
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .setCategory(NotificationCompat.CATEGORY_CALL)
            .apply {
                if (progress > 0) {
                    setProgress(100, progress, false)
                }
            }
            .build()
    }

    private fun ensureChannel(context: Context) {
        val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        if (nm.getNotificationChannel(CHANNEL_ID) != null) return

        val channel = NotificationChannel(
            CHANNEL_ID,
            "鹏Agent 灵动岛 (Live Update)",
            NotificationManager.IMPORTANCE_HIGH
        ).apply {
            description = "Agent任务灵动岛进度"
            setShowBadge(false)
            setSound(null, null)
            enableVibration(false)
        }
        nm.createNotificationChannel(channel)
    }
}
