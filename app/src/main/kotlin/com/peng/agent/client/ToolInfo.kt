package com.peng.agent.client

import com.google.gson.annotations.SerializedName

data class ToolInfo(
    val name: String,
    val description: String,
    val enabled: Boolean,
    @SerializedName("tool_type")
    val toolType: String,
    val callCount: Long = 0L,
    val path: String? = null,
    val available: Boolean = true
)
