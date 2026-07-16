package com.peng.agent.client

data class ContentPart(
    val type: String,
    val text: String? = null,
    val imageUrl: ImageUrl? = null,
    val videoUrl: VideoUrlData? = null
)
