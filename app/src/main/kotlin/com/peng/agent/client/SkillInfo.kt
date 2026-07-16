package com.peng.agent.client

import com.google.gson.annotations.SerializedName

data class SkillInfo(
    val name: String,
    val version: String? = "1.0.0",
    val author: String? = "",
    val description: String? = "",
    val tags: List<String>? = emptyList(),
    val content: String? = "",
    val enabled: Boolean = true,
    val source: String? = "user",
    val createdAt: String? = "",
    val updatedAt: String? = "",
    @SerializedName("has_references")
    val hasReferences: Boolean = false,
    @SerializedName("reference_count")
    val referenceCount: Int = 0,
    @SerializedName("prompt")
    val prompt: String? = "",
    @SerializedName("type")
    val skillType: String? = "document",
    val callCount: Long = 0L,
    val references: List<String>? = emptyList()
)
