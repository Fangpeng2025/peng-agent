package com.peng.agent.client

data class ToolCall(
    val id: String,
    val name: String,
    val arguments: String,
    val result: String? = null,
    val status: ToolCallStatus = ToolCallStatus.PENDING,
    val timestamp: Long = System.currentTimeMillis(),
    val contentParts: List<ContentPart> = emptyList()
)
