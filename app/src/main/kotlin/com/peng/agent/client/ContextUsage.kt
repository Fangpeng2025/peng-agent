package com.peng.agent.client

data class ContextUsage(
    val estimatedTokens: Long = 0L,
    val systemTokens: Long = 0L,
    val userTokens: Long = 0L,
    val assistantTokens: Long = 0L,
    val toolTokens: Long = 0L,
    val messageCount: Int = 0,
    val conversationId: String = ""
) {
    fun usagePercent(contextWindow: Int): Float =
        if (contextWindow > 0) (estimatedTokens.toFloat() / contextWindow) * 100f else 0f

    fun willCompress(contextWindow: Int, threshold: Float): Boolean =
        usagePercent(contextWindow) >= 100f * threshold

    fun compressionTriggerAt(contextWindow: Int, threshold: Float): Long =
        (contextWindow * threshold).toLong()

    fun formatUsage(contextWindow: Int): String {
        val pct = usagePercent(contextWindow)
        return "已用 %,d / %,d (%.1f%%)".format(estimatedTokens, contextWindow, pct)
    }
}
