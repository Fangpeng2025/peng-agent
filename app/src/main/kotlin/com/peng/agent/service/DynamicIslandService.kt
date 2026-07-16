package com.peng.agent.service

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.IBinder
import android.util.Log
import androidx.core.app.NotificationCompat
import com.peng.agent.MainActivity
import com.peng.agent.R

/**
 * 灵动岛服务 — 在通知栏显示任务进度
 *
 * 支持两种模式：
 * 1. Live Update (API 36+): 使用 promoted notification 实现灵动岛
 * 2. 常规通知 (API 29+): 使用前台通知显示任务进度
 */
class DynamicIslandService : Service() {

    companion object {
        private const val TAG = "DynamicIsland"
        private const val CHANNEL_ID = "peng_agent_island"
        private const val NOTIFICATION_ID = 2001

        const val ACTION_START = "com.peng.agent.ACTION_ISLAND_START"
        const val ACTION_UPDATE = "com.peng.agent.ACTION_ISLAND_UPDATE"
        const val ACTION_STOP = "com.peng.agent.ACTION_ISLAND_STOP"

        const val EXTRA_TASK_ID = "task_id"
        const val EXTRA_TASK_NAME = "task_name"
        const val EXTRA_PHASE = "phase"
        const val EXTRA_TOOL_NAME = "tool_name"
        const val EXTRA_TOOL_STATUS = "tool_status"
        const val EXTRA_PROGRESS = "progress"
    }

    private var currentTaskId: String = ""
    private var currentTaskName: String = ""

    // ── Lifecycle ──────────────────────────────────────────────────────────

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
        Log.i(TAG, "✅ 灵动岛服务已创建")
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_START -> {
                val taskId = intent.getStringExtra(EXTRA_TASK_ID) ?: ""
                val taskName = intent.getStringExtra(EXTRA_TASK_NAME) ?: "任务执行中"
                startIsland(taskId, taskName)
            }
            ACTION_UPDATE -> {
                val phase = intent.getStringExtra(EXTRA_PHASE) ?: ""
                val toolName = intent.getStringExtra(EXTRA_TOOL_NAME) ?: ""
                val toolStatus = intent.getStringExtra(EXTRA_TOOL_STATUS) ?: ""
                val progress = intent.getIntExtra(EXTRA_PROGRESS, 0)
                updateIsland(phase, toolName, toolStatus, progress)
            }
            ACTION_STOP -> {
                stopIsland()
            }
        }
        return START_NOT_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null

    // ── Island control ─────────────────────────────────────────────────────

    private fun startIsland(taskId: String, taskName: String) {
        currentTaskId = taskId
        currentTaskName = taskName
        Log.i(TAG, "🏝️ 启动灵动岛: $taskName ($taskId)")

        val notification = buildNotification(
            title = taskName,
            content = "思考中...",
            phase = "thinking"
        )
        startForeground(NOTIFICATION_ID, notification)

        // Try to promote to Live Update on API 36+
        if (Build.VERSION.SDK_INT >= 36) {
            try {
                promoteToLiveUpdate()
            } catch (e: Exception) {
                Log.w(TAG, "灵动岛升级失败，使用常规通知", e)
            }
        }
    }

    private fun updateIsland(phase: String, toolName: String, toolStatus: String, progress: Int) {
        if (currentTaskId.isEmpty()) return

        val content = when (phase) {
            "thinking" -> "思考中..."
            "tool_call" -> "执行工具: $toolName ($toolStatus)"
            "streaming" -> "回复中..."
            else -> "处理中..."
        }

        val notification = buildNotification(
            title = currentTaskName,
            content = content,
            phase = phase,
            progress = progress
        )

        val nm = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        nm.notify(NOTIFICATION_ID, notification)
    }

    private fun stopIsland() {
        Log.i(TAG, "🏝️ 停止灵动岛: $currentTaskName")
        currentTaskId = ""
        currentTaskName = ""
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    // ── Notification builder ───────────────────────────────────────────────

    private fun buildNotification(
        title: String,
        content: String,
        phase: String,
        progress: Int = 0
    ): Notification {
        val intent = Intent(this, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP
            putExtra("return_session_id", currentTaskId)
        }
        val pendingIntent = PendingIntent.getActivity(
            this, 0, intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(R.mipmap.ic_launcher)
            .setContentTitle(title)
            .setContentText(content)
            .setOngoing(true)
            .setSilent(true)
            .setContentIntent(pendingIntent)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .setCategory(NotificationCompat.CATEGORY_PROGRESS)
            .apply {
                if (progress > 0) {
                    setProgress(100, progress, false)
                }
            }
            .build()
    }

    // ── Live Update (API 36+) ──────────────────────────────────────────────

    private fun promoteToLiveUpdate() {
        if (Build.VERSION.SDK_INT >= 36) {
            val nm = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
            if (nm.canPostPromotedNotifications()) {
                Log.i(TAG, "🏝️ 升级为 Live Update 灵动岛")
                // On API 36+, use a promoted notification style for the dynamic island
                val promotedNotification = NotificationCompat.Builder(this, CHANNEL_ID)
                    .setSmallIcon(R.mipmap.ic_launcher)
                    .setContentTitle(currentTaskName)
                    .setContentText("思考中...")
                    .setOngoing(true)
                    .setSilent(true)
                    .setCategory(NotificationCompat.CATEGORY_CALL)
                    .setPriority(NotificationCompat.PRIORITY_HIGH)
                    .build()
                nm.notify(NOTIFICATION_ID, promotedNotification)
            } else {
                Log.w(TAG, "🏝️ 设备不支持 promoted notifications")
            }
        }
    }

    // ── Notification channel ───────────────────────────────────────────────

    private fun createNotificationChannel() {
        val nm = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        if (nm.getNotificationChannel(CHANNEL_ID) != null) return

        val channel = NotificationChannel(
            CHANNEL_ID,
            "鹏Agent 灵动岛",
            NotificationManager.IMPORTANCE_LOW
        ).apply {
            description = "显示Agent任务执行进度"
            setShowBadge(false)
            setSound(null, null)
            enableVibration(false)
        }
        nm.createNotificationChannel(channel)
    }
}
