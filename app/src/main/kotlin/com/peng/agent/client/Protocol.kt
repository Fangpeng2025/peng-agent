package com.peng.agent.client

// ── Enums ────────────────────────────────────────────────────────────────────

enum class StepType {
    THINKING, TOOL_CALL, TOOL_RESULT, STREAMING, ERROR
}

enum class AttachmentType {
    IMAGE, VIDEO, FILE, AUDIO
}

enum class ConnectionState {
    DISCONNECTED, CONNECTING, CONNECTED, ERROR
}

enum class ToolCallStatus {
    PENDING, RUNNING, COMPLETED, FAILED
}

// ── Tiny data classes ────────────────────────────────────────────────────────

data class ImageUrl(
    val url: String
)

data class VideoUrlData(
    val url: String,
    val durationSec: Double = 0.0,
    val sizeMb: Double = 0.0,
    val width: Int = 0,
    val height: Int = 0
)
