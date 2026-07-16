package com.peng.agent.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.Error
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.peng.agent.client.BackendClient
import com.peng.agent.ui.components.ModernCard
import com.peng.agent.ui.components.ModernGroupCard
import com.peng.agent.ui.components.ModernListItem
import com.peng.agent.ui.theme.BrandPrimary

@Composable
fun BackendStatusScreen(
    client: BackendClient,
    modifier: Modifier = Modifier
) {
    val connectionState by client.connectionState.collectAsState()
    val config by client.config.collectAsState()

    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 16.dp, vertical = 8.dp)
    ) {
        Text(
            text = "后端状态",
            style = MaterialTheme.typography.headlineLarge,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.padding(bottom = 16.dp)
        )

        // Connection status
        ModernCard(onClick = null, modifier = Modifier.fillMaxWidth()) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(16.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Icon(
                    imageVector = if (client.isRunning()) Icons.Default.CheckCircle else Icons.Default.Error,
                    contentDescription = null,
                    modifier = Modifier.size(24.dp),
                    tint = if (client.isRunning()) BrandPrimary else MaterialTheme.colorScheme.error
                )
                Spacer(modifier = Modifier.width(12.dp))
                Column {
                    Text(
                        text = if (client.isRunning()) "后端运行中" else "后端未运行",
                        style = MaterialTheme.typography.titleMedium,
                        color = MaterialTheme.colorScheme.onSurface
                    )
                    Text(
                        text = "连接状态: $connectionState",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
        }

        Spacer(modifier = Modifier.height(16.dp))

        // Backend info
        ModernGroupCard(title = "后端信息") {
            ModernListItem(
                title = "模型",
                subtitle = config.model,
                onClick = { }
            )
            ModernListItem(
                title = "API地址",
                subtitle = config.apiBase,
                onClick = { }
            )
            ModernListItem(
                title = "最大Token",
                subtitle = "${config.maxTokens}",
                onClick = { }
            )
            ModernListItem(
                title = "运行状态",
                subtitle = client.getStatus().take(100),
                onClick = { }
            )
        }

        Spacer(modifier = Modifier.height(32.dp))
    }
}
