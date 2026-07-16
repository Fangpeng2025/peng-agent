package com.peng.agent.client

data class SessionInfo(
    val id: String,
    val title: String,
    val createdAt: Long,
    val messageCount: Int
)
