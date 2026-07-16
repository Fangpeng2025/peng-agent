package com.peng.agent.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Psychology
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.peng.agent.client.BackendClient
import com.peng.agent.ui.components.ModernEmptyState
import com.peng.agent.ui.components.ModernGroupCard
import com.peng.agent.ui.components.ModernListItem
import com.peng.agent.ui.theme.BrandPrimary

@Composable
fun MemoryPage(
    client: BackendClient,
    modifier: Modifier = Modifier
) {
    val config by client.config.collectAsState()

    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 16.dp, vertical = 8.dp)
    ) {
        Text(
            text = "记忆",
            style = MaterialTheme.typography.headlineLarge,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.padding(bottom = 16.dp)
        )

        ModernGroupCard(title = "记忆配置") {
            ModernListItem(
                icon = Icons.Default.Search,
                title = "搜索限制",
                subtitle = "每次检索 ${config.memorySearchLimit} 条记忆",
                onClick = { /* TODO */ }
            )
            ModernListItem(
                icon = Icons.Default.Psychology,
                title = "压缩策略",
                subtitle = config.compressionStrategy,
                onClick = { /* TODO */ }
            )
            ModernListItem(
                icon = Icons.Default.Psychology,
                title = "压缩阈值",
                subtitle = "${(config.compressionThreshold * 100).toInt()}%",
                onClick = { /* TODO */ }
            )
        }

        Spacer(modifier = Modifier.height(16.dp))

        ModernEmptyState(
            icon = Icons.Default.Psychology,
            title = "长期记忆",
            description = "Agent会自动从对话中提取和存储重要信息。\n记忆将在对话时自动检索。",
            actionLabel = "查看记忆",
            onAction = { /* TODO */ }
        )
    }
}
