package com.peng.agent.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import com.peng.agent.client.BackendClient
import com.peng.agent.client.ContextUsage
import com.peng.agent.ui.components.ModernCard
import com.peng.agent.ui.theme.BrandPrimary

@Composable
fun ContextUsagePanel(
    client: BackendClient,
    modifier: Modifier = Modifier
) {
    val contextUsage by client.contextUsage.collectAsState()
    val config by client.config.collectAsState()

    val usagePercent = contextUsage.usagePercent(config.contextWindow)
    val willCompress = contextUsage.willCompress(config.contextWindow, config.compressionThreshold)

    ModernCard(onClick = null, modifier = modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = "上下文用量",
                    style = MaterialTheme.typography.titleSmall,
                    color = MaterialTheme.colorScheme.onSurface,
                    modifier = Modifier.weight(1f)
                )
                Text(
                    text = contextUsage.formatUsage(config.contextWindow),
                    style = MaterialTheme.typography.labelSmall,
                    color = if (usagePercent > 80) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant
                )
            }

            LinearProgressIndicator(
                progress = { (usagePercent / 100f).coerceIn(0f, 1f) },
                modifier = Modifier
                    .fillMaxWidth()
                    .height(4.dp)
                    .clip(RoundedCornerShape(2.dp)),
                color = when {
                    usagePercent > 90 -> MaterialTheme.colorScheme.error
                    usagePercent > 70 -> MaterialTheme.colorScheme.secondary
                    else -> BrandPrimary
                },
                trackColor = MaterialTheme.colorScheme.outlineVariant
            )

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                TokenStat("用户", contextUsage.userTokens)
                TokenStat("助手", contextUsage.assistantTokens)
                TokenStat("消息", contextUsage.messageCount.toLong())
            }

            if (willCompress) {
                Text(
                    text = "⚠️ 上下文即将压缩 (阈值: ${(config.compressionThreshold * 100).toInt()}%)",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.secondary
                )
            }
        }
    }
}

@Composable
private fun TokenStat(label: String, tokens: Long) {
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Text(
            text = formatTokens(tokens),
            style = MaterialTheme.typography.labelMedium,
            color = BrandPrimary
        )
        Text(
            text = label,
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}

private fun formatTokens(tokens: Long): String = when {
    tokens < 1000 -> "$tokens"
    tokens < 1000000 -> "%.1fk".format(tokens / 1000.0)
    else -> "%.1fM".format(tokens / 1000000.0)
}
