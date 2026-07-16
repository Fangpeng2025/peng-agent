package com.peng.agent.service

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.os.IBinder
import android.util.Log
import androidx.core.app.NotificationCompat
import com.peng.agent.MainActivity
import com.peng.agent.R

/**
 * 任务进度服务 — 前台通知显示Agent任务执行进度
 *
 * 当Agent执行长时间任务时，通过前台通知保持服务存活并显示进度。
 */
class TaskProgressService : Service() {

    companion object {
        private const val TAG = "TaskProgress"
        private const val CHANNEL_ID = "peng_agent_progress"
        private const val NOTIFICATION_ID = 1001

        const val ACTION_START = "com.peng.agent.ACTION_PROGRESS_START"
        const val ACTION_UPDATE = "com.peng.agent.ACTION_PROGRESS_UPDATE"
        const val ACTION_STOP = "com.peng.agent.ACTION_PROGRESS_STOP"

        const val EXTRA_TASK_ID = "task_id"
        const val EXTRA_TASK_NAME = "task_name"
        const val EXTRA_MESSAGE = "message"
        const val EXTRA_PROGRESS = "progress"
    }

    private var currentTaskId: String = ""
    private var currentTaskName: String = ""

    // ── Lifecycle ──────────────────────────────────────────────────────────

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
        Log.i(TAG, "✅ 任务进度服务已创建")
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_START -> {
                val taskId = intent.getStringExtra(EXTRA_TASK_ID) ?: ""
                val taskName = intent.getStringExtra(EXTRA_TASK_NAME) ?: "任务执行中"
                startProgress(taskId, taskName)
            }
            ACTION_UPDATE -> {
                val message = intent.getStringExtra(EXTRA_MESSAGE) ?: ""
                val progress = intent.getIntExtra(EXTRA_PROGRESS, 0)
                updateProgress(message, progress)
            }
            ACTION_STOP -> {
                stopProgress()
            }
        }
        return START_NOT_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null

    // ── Progress control ───────────────────────────────────────────────────

    private fun startProgress(taskId: String, taskName: String) {
        currentTaskId = taskId
        currentTaskName = taskName
        Log.i(TAG, "📋 启动任务进度: $taskName ($taskId)")

        val notification = buildNotification(
            title = taskName,
            content = "准备中...",
            progress = 0
        )
        startForeground(NOTIFICATION_ID, notification)
    }

    private fun updateProgress(message: String, progress: Int) {
        if (currentTaskId.isEmpty()) return

        val notification = buildNotification(
            title = currentTaskName,
            content = message,
            progress = progress
        )
        val nm = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        nm.notify(NOTIFICATION_ID, notification)
    }

    private fun stopProgress() {
        Log.i(TAG, "📋 停止任务进度: $currentTaskName")
        currentTaskId = ""
        currentTaskName = ""
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    // ── Notification builder ───────────────────────────────────────────────

    private fun buildNotification(title: String, content: String, progress: Int): Notification {
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
            .setContentIntent(pendingIntent)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .setCategory(NotificationCompat.CATEGORY_PROGRESS)
            .apply {
                if (progress > 0 && progress < 100) {
                    setProgress(100, progress, false)
                }
            }
            .build()
    }

    // ── Notification channel ───────────────────────────────────────────────

    private fun createNotificationChannel() {
        val nm = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        if (nm.getNotificationChannel(CHANNEL_ID) != null) return

        val channel = NotificationChannel(
            CHANNEL_ID,
            "鹏Agent 任务进度",
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
