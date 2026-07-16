package com.peng.agent.client

data class ChatMessage(
    val id: String = java.util.UUID.randomUUID().toString(),
    val text: String = "",
    val isUser: Boolean = false,
    val isStreaming: Boolean = false,
    val toolCalls: List<ToolCall> = emptyList(),
    val toolExecutionTime: Long? = null,
    val errorMessage: String? = null,
    val attachments: List<Attachment> = emptyList(),
    val contentParts: List<ContentPart> = emptyList(),
    val timestamp: Long = System.currentTimeMillis(),
    val currentToolCall: ToolCall? = null,
    val steps: List<AgentStep> = emptyList()
)
