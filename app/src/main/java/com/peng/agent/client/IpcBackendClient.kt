package com.peng.agent.client

import android.content.Context
import android.util.Log
import com.peng.agent.ipc.PengDaemonClient
import com.peng.agent.ipc.PhoneCallbackHandler
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*
import org.json.JSONObject
import java.util.UUID

/**
 * 基于 IPC 的 BackendClient 实现
 * 通过 Unix Socket 与 peng-daemon 通信
 */
class IpcBackendClient(
    private val context: Context
) : BackendClient {
    
    companion object {
        private const val TAG = "IpcBackendClient"
    }
    
    private val daemonClient = PengDaemonClient()
    private val phoneHandler = PhoneCallbackHandler(context, daemonClient)
    
    // ── StateFlows ─────────────────────────────────────────────────────────
    
    private val _config = MutableStateFlow(AppConfig())
    override val config: StateFlow<AppConfig> = _config.asStateFlow()
    
    private val _connectionState = MutableStateFlow(ConnectionState.DISCONNECTED)
    override val connectionState: StateFlow<ConnectionState> = _connectionState.asStateFlow()
    
    private val _contextUsage = MutableStateFlow(ContextUsage())
    override val contextUsage: StateFlow<ContextUsage> = _contextUsage.asStateFlow()
    
    private val _currentMessages = MutableStateFlow<List<ChatMessage>>(emptyList())
    override val currentMessages: StateFlow<List<ChatMessage>> = _currentMessages.asStateFlow()
    
    private val _currentSessionId = MutableStateFlow("")
    override val currentSessionId: StateFlow<String> = _currentSessionId.asStateFlow()
    
    private val _isStreaming = MutableStateFlow(false)
    override val isStreaming: StateFlow<Boolean> = _isStreaming.asStateFlow()
    
    private val _sessions = MutableStateFlow<List<SessionInfo>>(emptyList())
    override val sessions: StateFlow<List<SessionInfo>> = _sessions.asStateFlow()
    
    private val _skills = MutableStateFlow<List<SkillInfo>>(emptyList())
    override val skills: StateFlow<List<SkillInfo>> = _skills.asStateFlow()
    
    private val _taskReturnSessionId = MutableStateFlow("")
    override val taskReturnSessionId: StateFlow<String> = _taskReturnSessionId.asStateFlow()
    
    private val _tools = MutableStateFlow<List<ToolInfo>>(emptyList())
    override val tools: StateFlow<List<ToolInfo>> = _tools.asStateFlow()
    
    // ── Connection ─────────────────────────────────────────────────────────
    
    override fun connect() {
        CoroutineScope(Dispatchers.IO).launch {
            _connectionState.value = ConnectionState.CONNECTING
            
            val result = daemonClient.connect()
            
            if (result.isSuccess) {
                _connectionState.value = ConnectionState.CONNECTED
                Log.i(TAG, "Connected to daemon")
            } else {
                _connectionState.value = ConnectionState.ERROR
                Log.e(TAG, "Failed to connect: ${result.exceptionOrNull()?.message}")
            }
        }
    }
    
    // ── Chat ───────────────────────────────────────────────────────────────
    
    override suspend fun streamChat(
        message: String,
        history: List<ChatMessage>,
        onChunk: (String) -> Unit,
        onToolCall: ((ToolCall) -> Unit)?
    ): Result<String> = withContext(Dispatchers.IO) {
        if (!daemonClient.isConnected()) {
            return@withContext Result.failure<String>(Exception("Not connected to daemon"))
        }
        
        _isStreaming.value = true
        val fullResponse = StringBuilder()
        
        try {
            daemonClient.chat(message, history.map { msg ->
                PengDaemonClient.ChatMessage(
                    role = if (msg.isUser) "user" else "assistant",
                    content = msg.text
                )
            }).collect { event ->
                when (event) {
                    is PengDaemonClient.StreamEvent.Token -> {
                        fullResponse.append(event.token)
                        onChunk(event.token)
                    }
                    is PengDaemonClient.StreamEvent.ToolStart -> {
                        onToolCall?.invoke(ToolCall(
                            id = UUID.randomUUID().toString(),
                            name = event.name,
                            arguments = event.args
                        ))
                    }
                    is PengDaemonClient.StreamEvent.ToolEnd -> {
                        // 工具执行完成
                    }
                    is PengDaemonClient.StreamEvent.PhoneCallback -> {
                        val result = phoneHandler.handleCallback(event)
                        Log.i(TAG, "Phone callback result: $result")
                    }
                    is PengDaemonClient.StreamEvent.Complete -> {
                        _isStreaming.value = false
                    }
                    is PengDaemonClient.StreamEvent.Error -> {
                        _isStreaming.value = false
                    }
                }
            }
            
            Result.success(fullResponse.toString())
        } catch (e: Exception) {
            _isStreaming.value = false
            Result.failure(e)
        }
    }
    
    override fun abortStream() {
        CoroutineScope(Dispatchers.IO).launch {
            daemonClient.abort()
        }
    }
    
    // ── Sessions ───────────────────────────────────────────────────────────
    
    override fun newSession(title: String): SessionInfo {
        val session = SessionInfo(
            id = UUID.randomUUID().toString(),
            title = title,
            createdAt = System.currentTimeMillis(),
            messageCount = 0
        )
        _sessions.value = _sessions.value + session
        _currentSessionId.value = session.id
        _currentMessages.value = emptyList()
        return session
    }
    
    override fun createSession(title: String): SessionInfo = newSession(title)
    
    override fun setCurrentSession(sessionId: String) {
        if (_sessions.value.any { it.id == sessionId }) {
            _currentSessionId.value = sessionId
        }
    }
    
    override suspend fun switchSession(sessionId: String) {
        setCurrentSession(sessionId)
    }
    
    override fun deleteSession(sessionId: String): Boolean {
        _sessions.value = _sessions.value.filter { it.id != sessionId }
        if (_currentSessionId.value == sessionId) {
            _currentSessionId.value = ""
            _currentMessages.value = emptyList()
        }
        return true
    }
    
    override fun listSessions(): List<SessionInfo> = _sessions.value
    
    override fun getCurrentSessionId(): String = _currentSessionId.value
    
    override fun saveSessionHistory(sessionId: String, messages: List<ChatMessage>) {
        // TODO: 持久化会话历史
    }
    
    // ── Tools ──────────────────────────────────────────────────────────────
    
    override fun callTool(toolName: String, params: Map<String, Any>): String {
        val paramsJson = JSONObject(params)
        val result = runBlocking {
            daemonClient.callTool(toolName, paramsJson)
        }
        
        return result.fold(
            onSuccess = { it.toString() },
            onFailure = { """{"error": "${it.message}"}""" }
        )
    }
    
    override fun listTools(): List<ToolInfo> = _tools.value
    
    override fun toggleTool(toolName: String, enabled: Boolean) {
        // TODO: 实现工具开关
    }
    
    // ── Skills ─────────────────────────────────────────────────────────────
    
    override fun listSkills(): List<SkillInfo> = _skills.value
    
    override fun getSkill(skillName: String): SkillInfo? = _skills.value.find { it.name == skillName }
    
    override fun createSkill(skill: SkillInfo): Boolean {
        _skills.value = _skills.value + skill
        return true
    }
    
    override fun updateSkill(skill: SkillInfo): Boolean {
        _skills.value = _skills.value.map { if (it.name == skill.name) skill else it }
        return true
    }
    
    override fun deleteSkill(skillName: String): Boolean {
        _skills.value = _skills.value.filter { it.name != skillName }
        return true
    }
    
    override fun toggleSkill(skillName: String, enabled: Boolean) {
        _skills.value = _skills.value.map { 
            if (it.name == skillName) it.copy(enabled = enabled) else it 
        }
    }
    
    // ── Config ─────────────────────────────────────────────────────────────
    
    override fun getConfig(): AppConfig = _config.value
    
    override fun setConfig(config: AppConfig) {
        _config.value = config
    }
    
    override fun setConfig(key: String, value: Any) {
        // TODO: 实现单键配置更新
    }
    
    override fun resetConfig() {
        _config.value = AppConfig()
    }
    
    // ── Execution ──────────────────────────────────────────────────────────
    
    override fun executeShell(command: String, timeout: Long): String {
        return callTool("execute_shell", mapOf("command" to command))
    }
    
    override fun executePython(code: String, timeout: Long): String {
        return callTool("execute_python", mapOf("code" to code))
    }
    
    // ── Misc ───────────────────────────────────────────────────────────────
    
    override fun initialize(context: Context): Boolean {
        connect()
        return true
    }
    
    override fun isRunning(): Boolean = daemonClient.isConnected()
    
    override fun getStatus(): String {
        return if (daemonClient.isConnected()) "connected" else "disconnected"
    }
    
    override fun getContextUsage(): ContextUsage = _contextUsage.value
    
    override fun diagnoseKnowledge(): String = "Not implemented"
    
    override fun updateCurrentMessages(messages: List<ChatMessage>) {
        _currentMessages.value = messages
    }
    
    override fun shutdown() {
        daemonClient.disconnect()
        _connectionState.value = ConnectionState.DISCONNECTED
    }
}