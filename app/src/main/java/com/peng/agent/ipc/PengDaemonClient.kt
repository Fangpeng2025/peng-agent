package com.peng.agent.ipc

import android.net.LocalSocket
import android.net.LocalSocketAddress
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.flow.flowOn
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.*

/**
 * Peng Daemon IPC 客户端
 * 通过 Unix Socket 与 peng-daemon 通信
 */
class PengDaemonClient {
    
    companion object {
        private const val TAG = "PengDaemonClient"
        const val SOCKET_PATH = "/data/data/com.peng.agent/files/peng.sock"
    }
    
    private var socket: LocalSocket? = null
    private var outputStream: OutputStream? = null
    private var inputStream: InputStream? = null
    private var requestId = 0L
    
    /**
     * 连接到 daemon
     */
    suspend fun connect(): Result<Unit> = withContext(Dispatchers.IO) {
        try {
            socket = LocalSocket()
            socket!!.connect(LocalSocketAddress(SOCKET_PATH))
            
            outputStream = socket!!.getOutputStream()
            inputStream = socket!!.getInputStream()
            
            Log.i(TAG, "Connected to peng-daemon")
            Result.success(Unit)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to connect to daemon", e)
            Result.failure(e)
        }
    }
    
    /**
     * 断开连接
     */
    fun disconnect() {
        try {
            outputStream?.close()
            inputStream?.close()
            socket?.close()
        } catch (e: Exception) {
            Log.e(TAG, "Error closing connection", e)
        }
        
        outputStream = null
        inputStream = null
        socket = null
        Log.i(TAG, "Disconnected from daemon")
    }
    
    /**
     * 检查是否已连接
     */
    fun isConnected(): Boolean = socket?.isConnected == true
    
    /**
     * 发送聊天消息，返回事件流
     */
    fun chat(message: String, history: List<ChatMessage>): Flow<StreamEvent> = flow {
        val id = nextRequestId()
        val request = JSONObject().apply {
            put("id", id)
            put("method", "chat")
            put("params", JSONObject().apply {
                put("message", message)
                put("history", history.map { msg ->
                    JSONObject().apply {
                        put("role", msg.role)
                        put("content", msg.content)
                    }
                })
            })
        }
        
        sendRequest(request)
        
        // 读取事件流
        val reader = inputStream?.bufferedReader()
        if (reader != null) {
            while (true) {
                val line = reader.readLine() ?: break
                val event = parseEvent(line)
                emit(event)
                
                if (event is StreamEvent.Complete || event is StreamEvent.Error) {
                    break
                }
            }
        }
    }.flowOn(Dispatchers.IO)
    
    /**
     * 调用工具
     */
    suspend fun callTool(name: String, params: JSONObject): Result<JSONObject> = withContext(Dispatchers.IO) {
        try {
            val id = nextRequestId()
            val request = JSONObject().apply {
                put("id", id)
                put("method", "call_tool")
                put("params", JSONObject().apply {
                    put("name", name)
                    put("params", params)
                })
            }
            
            sendRequest(request)
            
            val response = readResponse()
            if (response.has("error")) {
                Result.failure(Exception(response.getString("error")))
            } else {
                Result.success(response.getJSONObject("result"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
    
    /**
     * 发送 Phone 工具回调结果
     */
    suspend fun sendCallbackResult(callbackId: String, result: JSONObject): Result<Unit> = withContext(Dispatchers.IO) {
        try {
            val request = JSONObject().apply {
                put("id", nextRequestId())
                put("method", "callback_result")
                put("params", JSONObject().apply {
                    put("callback_id", callbackId)
                    put("result", result)
                })
            }
            
            sendRequest(request)
            Result.success(Unit)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
    
    /**
     * 获取状态
     */
    suspend fun getStatus(): Result<JSONObject> = withContext(Dispatchers.IO) {
        try {
            val request = JSONObject().apply {
                put("id", nextRequestId())
                put("method", "get_status")
            }
            
            sendRequest(request)
            val response = readResponse()
            Result.success(response)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
    
    /**
     * 中止当前对话
     */
    suspend fun abort(): Result<Unit> = withContext(Dispatchers.IO) {
        try {
            val request = JSONObject().apply {
                put("id", nextRequestId())
                put("method", "abort")
            }
            
            sendRequest(request)
            Result.success(Unit)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
    
    // ── Private Methods ─────────────────────────────────────────────────────
    
    private fun nextRequestId(): Long {
        return ++requestId
    }
    
    private fun sendRequest(request: JSONObject) {
        outputStream?.let { out ->
            out.write((request.toString() + "\n").toByteArray())
            out.flush()
        } ?: throw IOException("Not connected")
    }
    
    private fun readResponse(): JSONObject {
        val line = inputStream?.bufferedReader()?.readLine()
            ?: throw IOException("Connection closed")
        return JSONObject(line)
    }
    
    private fun parseEvent(line: String): StreamEvent {
        val json = JSONObject(line)
        val type = json.getString("type")
        
        return when (type) {
            "token" -> StreamEvent.Token(json.getString("data"))
            "tool_start" -> StreamEvent.ToolStart(
                json.getString("name"),
                json.optString("args", "")
            )
            "tool_end" -> StreamEvent.ToolEnd(
                json.getString("name"),
                json.getString("result")
            )
            "phone_callback" -> StreamEvent.PhoneCallback(
                json.getString("callback_id"),
                json.getString("tool"),
                json.getJSONObject("params")
            )
            "complete" -> StreamEvent.Complete(json.getString("data"))
            "error" -> StreamEvent.Error(json.getString("message"))
            else -> StreamEvent.Error("Unknown event type: $type")
        }
    }
    
    // ── Data Classes ─────────────────────────────────────────────────────────
    
    /**
     * 聊天消息
     */
    data class ChatMessage(
        val role: String,
        val content: String
    )
    
    /**
     * 流式事件
     */
    sealed class StreamEvent {
        data class Token(val token: String) : StreamEvent()
        data class ToolStart(val name: String, val args: String) : StreamEvent()
        data class ToolEnd(val name: String, val result: String) : StreamEvent()
        data class PhoneCallback(
            val callbackId: String,
            val tool: String,
            val params: JSONObject
        ) : StreamEvent()
        data class Complete(val response: String) : StreamEvent()
        data class Error(val message: String) : StreamEvent()
    }
}