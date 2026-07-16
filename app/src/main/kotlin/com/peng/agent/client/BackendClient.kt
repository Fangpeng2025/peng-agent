package com.peng.agent.client

import android.content.Context
import kotlinx.coroutines.flow.StateFlow
import kotlin.Result

interface BackendClient {

    val config: StateFlow<AppConfig>
    val connectionState: StateFlow<ConnectionState>
    val contextUsage: StateFlow<ContextUsage>
    val currentMessages: StateFlow<List<ChatMessage>>
    val currentSessionId: StateFlow<String>
    val isStreaming: StateFlow<Boolean>
    val sessions: StateFlow<List<SessionInfo>>
    val skills: StateFlow<List<SkillInfo>>
    val taskReturnSessionId: StateFlow<String>
    val tools: StateFlow<List<ToolInfo>>

    fun abortStream()

    fun callTool(toolName: String, params: Map<String, Any>): String

    fun connect() {}

    fun createSession(title: String = "新会话"): SessionInfo

    fun createSkill(skill: SkillInfo): Boolean

    fun deleteSession(sessionId: String): Boolean

    fun deleteSkill(skillName: String): Boolean

    fun diagnoseKnowledge(): String

    fun executePython(code: String, timeout: Long = 30L): String

    fun executeShell(command: String, timeout: Long = 30L): String

    fun getConfig(): AppConfig

    fun getContextUsage(): ContextUsage

    fun getCurrentSessionId(): String

    fun getSkill(skillName: String): SkillInfo?

    fun getStatus(): String

    fun initialize(context: Context): Boolean

    fun isRunning(): Boolean

    fun listSessions(): List<SessionInfo>

    fun listSkills(): List<SkillInfo>

    fun listTools(): List<ToolInfo>

    fun newSession(title: String = "新会话"): SessionInfo

    fun resetConfig() {
        setConfig(AppConfig())
    }

    fun saveSessionHistory(sessionId: String, messages: List<ChatMessage>)

    fun setConfig(config: AppConfig)

    fun setConfig(key: String, value: Any) {}

    fun setCurrentSession(sessionId: String)

    fun shutdown()

    suspend fun streamChat(
        message: String,
        history: List<ChatMessage> = emptyList(),
        onChunk: (String) -> Unit,
        onToolCall: ((ToolCall) -> Unit)? = null
    ): Result<String>

    suspend fun switchSession(sessionId: String)

    fun toggleSkill(skillName: String, enabled: Boolean)

    fun toggleTool(toolName: String, enabled: Boolean)

    fun updateCurrentMessages(messages: List<ChatMessage>)

    fun updateSkill(skill: SkillInfo): Boolean
}
