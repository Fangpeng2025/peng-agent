package com.peng.agent.client

import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

data class MemoryEntry(
    val id: String,
    val key: String,
    val value: String,
    val category: String,
    val createdAt: Long,
    val score: Float = 0f
) {
    fun formatTime(): String =
        SimpleDateFormat("MM-dd HH:mm", Locale.getDefault()).format(Date(createdAt))

    fun preview(maxLen: Int = 100): String =
        if (value.length <= maxLen) value else value.take(maxLen) + "..."
}
