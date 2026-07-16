package com.peng.agent.client

data class Attachment(
    val uri: String,
    val type: AttachmentType,
    val name: String = "",
    val size: Long = 0L,
    val mimeType: String = "",
    val cachedDataUrl: String? = null
)
