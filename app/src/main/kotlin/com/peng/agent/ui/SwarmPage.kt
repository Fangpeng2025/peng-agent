package com.peng.agent.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Hub
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.peng.agent.client.BackendClient
import com.peng.agent.ui.components.ModernEmptyState
import com.peng.agent.ui.components.ModernGroupCard
import com.peng.agent.ui.components.ModernListItem

@Composable
fun SwarmPage(
    client: BackendClient,
    modifier: Modifier = Modifier
) {
    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 16.dp, vertical = 8.dp)
    ) {
        Text(
            text = "Swarm",
            style = MaterialTheme.typography.headlineLarge,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.padding(bottom = 16.dp)
        )

        ModernGroupCard(title = "多Agent协作") {
            ModernListItem(
                icon = Icons.Default.Hub,
                title = "Worker模型",
                subtitle = "子Agent使用的模型",
                onClick = { /* TODO */ }
            )
            ModernListItem(
                icon = Icons.Default.Hub,
                title = "最大轮数",
                subtitle = "Agent间最大交互轮数",
                onClick = { /* TODO */ }
            )
        }

        Spacer(modifier = Modifier.height(16.dp))

        ModernEmptyState(
            icon = Icons.Default.Hub,
            title = "Swarm 模式",
            description = "多Agent协作模式，允许多个Agent协同完成复杂任务。\n此功能正在开发中。",
            actionLabel = "创建Swarm",
            onAction = { /* TODO */ }
        )
    }
}
