package com.peng.agent.client

data class ChatUiState(
    val inputText: String = "",
    val pendingAttachments: List<Attachment> = emptyList(),
    val contextBarExpanded: Boolean = false,
    val showAttachMenu: Boolean = false,
    val errorMessage: String? = null,
    val showTopBar: Boolean = false
)
