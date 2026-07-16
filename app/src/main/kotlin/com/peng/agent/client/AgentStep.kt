package com.peng.agent.client

data class AgentStep(
    val id: String = java.util.UUID.randomUUID().toString(),
    val type: StepType,
    val text: String = "",
    val toolCallId: String = "",
    val toolName: String = "",
    val toolArguments: String = "",
    val toolResult: String = "",
    val isToolError: Boolean = false,
    val timestamp: Long = System.currentTimeMillis(),
    val isStreaming: Boolean = false
)
