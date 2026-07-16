package com.peng.agent.service

import android.content.Context
import android.content.SharedPreferences
import android.util.Log
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * 灵动岛状态管理 — 单例，在 MainActivity.onCreate 中初始化
 *
 * 持有当前任务的状态，供 UI 和 Service 共同读取。
 */
object DynamicIslandState {

    private const val TAG = "DynamicIslandState"
    private const val PREFS_NAME = "peng_island_state"

    private var prefs: SharedPreferences? = null

    // ── State ──────────────────────────────────────────────────────────────

    data class IslandState(
        val taskId: String = "",
        val taskName: String = "",
        val phase: String = "",          // thinking, tool_call, streaming, idle
        val toolName: String = "",
        val toolStatus: String = "",     // running, completed, error
        val progress: Int = 0,
        val isActive: Boolean = false,
        val startTime: Long = 0L
    )

    private val _state = MutableStateFlow(IslandState())
    val state: StateFlow<IslandState> = _state.asStateFlow()

    // ── Init ───────────────────────────────────────────────────────────────

    fun init(context: Context) {
        prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        Log.i(TAG, "✅ DynamicIslandState 已初始化")
    }

    // ── Public API ─────────────────────────────────────────────────────────

    fun startTask(taskId: String, taskName: String) {
        _state.value = IslandState(
            taskId = taskId,
            taskName = taskName,
            phase = "thinking",
            isActive = true,
            startTime = System.currentTimeMillis()
        )
        Log.i(TAG, "🏝️ 启动任务: $taskName ($taskId)")
    }

    fun updatePhase(phase: String) {
        _state.value = _state.value.copy(phase = phase)
        Log.d(TAG, "🏝️ 阶段更新: $phase")
    }

    fun updateToolProgress(toolName: String, status: String) {
        _state.value = _state.value.copy(
            phase = "tool_call",
            toolName = toolName,
            toolStatus = status
        )
        Log.d(TAG, "🏝️ 工具进度: $toolName → $status")
    }

    fun updateProgress(progress: Int) {
        _state.value = _state.value.copy(progress = progress)
    }

    fun stopTask() {
        _state.value = IslandState(isActive = false)
        Log.i(TAG, "🏝️ 任务已停止")
    }

    fun isActive(): Boolean = _state.value.isActive

    fun getCurrentTaskId(): String = _state.value.taskId

    // ── Elapsed time ───────────────────────────────────────────────────────

    fun getElapsedSeconds(): Long {
        val state = _state.value
        if (!state.isActive || state.startTime == 0L) return 0L
        return (System.currentTimeMillis() - state.startTime) / 1000
    }

    fun formatElapsed(): String {
        val seconds = getElapsedSeconds()
        val minutes = seconds / 60
        val secs = seconds % 60
        return if (minutes > 0) "${minutes}分${secs}秒" else "${secs}秒"
    }
}
