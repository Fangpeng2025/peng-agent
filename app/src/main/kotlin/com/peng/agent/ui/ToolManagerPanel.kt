package com.peng.agent.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Build
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.peng.agent.client.BackendClient
import com.peng.agent.client.ToolInfo
import com.peng.agent.ui.components.ModernCard
import com.peng.agent.ui.components.ModernGroupCard
import com.peng.agent.ui.theme.BrandPrimary
import com.peng.agent.ui.theme.BrandContainer
import com.peng.agent.util.ToolNameCn

@Composable
fun ToolManagerPanel(
    client: BackendClient,
    onToolClick: (ToolInfo) -> Unit,
    modifier: Modifier = Modifier
) {
    val tools by client.tools.collectAsState()

    val builtinTools = tools.filter { it.toolType == "builtin" }
    val multimediaTools = tools.filter { it.toolType == "multimedia" }
    val otherTools = tools.filter { it.toolType != "builtin" && it.toolType != "multimedia" }

    Column(modifier = modifier) {
        // Header
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = "工具管理",
                style = MaterialTheme.typography.headlineLarge,
                color = MaterialTheme.colorScheme.onSurface,
                modifier = Modifier.weight(1f)
            )
            Text(
                text = "${tools.filter { it.enabled }.size}/${tools.size}",
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }

        // Builtin tools
        if (builtinTools.isNotEmpty()) {
            ModernGroupCard(title = "内置工具 (${builtinTools.size})") {
                builtinTools.forEach { tool ->
                    ToolItemRow(
                        tool = tool,
                        onToggle = { enabled -> client.toggleTool(tool.name, enabled) },
                        onClick = { onToolClick(tool) }
                    )
                }
            }
            Spacer(modifier = Modifier.height(16.dp))
        }

        // Multimedia tools
        if (multimediaTools.isNotEmpty()) {
            ModernGroupCard(title = "多媒体工具 (${multimediaTools.size})") {
                multimediaTools.forEach { tool ->
                    ToolItemRow(
                        tool = tool,
                        onToggle = { enabled -> client.toggleTool(tool.name, enabled) },
                        onClick = { onToolClick(tool) }
                    )
                }
            }
            Spacer(modifier = Modifier.height(16.dp))
        }

        // Other tools
        if (otherTools.isNotEmpty()) {
            ModernGroupCard(title = "其他工具 (${otherTools.size})") {
                otherTools.forEach { tool ->
                    ToolItemRow(
                        tool = tool,
                        onToggle = { enabled -> client.toggleTool(tool.name, enabled) },
                        onClick = { onToolClick(tool) }
                    )
                }
            }
        }
    }
}

@Composable
private fun ToolItemRow(
    tool: ToolInfo,
    onToggle: (Boolean) -> Unit,
    onClick: () -> Unit
) {
    ModernCard(onClick = onClick, modifier = Modifier.fillMaxWidth()) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 10.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                imageVector = Icons.Default.Build,
                contentDescription = null,
                modifier = Modifier.size(20.dp),
                tint = if (tool.enabled) BrandPrimary else MaterialTheme.colorScheme.onSurfaceVariant
            )
            Spacer(modifier = Modifier.size(12.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = ToolNameCn.getCnName(tool.name),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurface,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
                Text(
                    text = tool.name,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
            }
            Surface(
                shape = RoundedCornerShape(4.dp),
                color = if (tool.enabled) BrandContainer else MaterialTheme.colorScheme.surfaceVariant,
                modifier = Modifier
            ) {
                Text(
                    text = if (tool.enabled) "启用" else "禁用",
                    style = MaterialTheme.typography.labelSmall,
                    color = if (tool.enabled) BrandPrimary else MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                )
            }
        }
    }
}
