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
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Build
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.peng.agent.client.ToolInfo
import com.peng.agent.ui.components.ModernCard
import com.peng.agent.ui.components.ModernGroupCard
import com.peng.agent.ui.components.ModernListItem
import com.peng.agent.ui.components.ModernSwitchItem
import com.peng.agent.ui.theme.BrandPrimary
import com.peng.agent.util.ToolNameCn

@Composable
fun ToolsDetailPage(
    tool: ToolInfo,
    onBack: () -> Unit,
    onToggle: (Boolean) -> Unit,
    modifier: Modifier = Modifier
) {
    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
    ) {
        // Top bar
        Surface(
            modifier = Modifier.fillMaxWidth(),
            color = MaterialTheme.colorScheme.surface
        ) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 8.dp, vertical = 4.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                IconButton(onClick = onBack) {
                    Icon(
                        imageVector = Icons.Default.ArrowBack,
                        contentDescription = "返回",
                        tint = BrandPrimary
                    )
                }
                Icon(
                    imageVector = Icons.Default.Build,
                    contentDescription = null,
                    modifier = Modifier.size(20.dp),
                    tint = if (tool.enabled) BrandPrimary else MaterialTheme.colorScheme.onSurfaceVariant
                )
                Spacer(modifier = Modifier.width(8.dp))
                Text(
                    text = ToolNameCn.getCnName(tool.name),
                    style = MaterialTheme.typography.titleLarge,
                    color = MaterialTheme.colorScheme.onSurface,
                    modifier = Modifier.weight(1f)
                )
            }
        }

        Spacer(modifier = Modifier.height(16.dp))

        // Tool info
        ModernGroupCard(title = "工具信息") {
            ModernListItem(
                title = "名称",
                subtitle = tool.name,
                onClick = { }
            )
            ModernListItem(
                title = "中文名",
                subtitle = ToolNameCn.getCnName(tool.name),
                onClick = { }
            )
            ModernListItem(
                title = "描述",
                subtitle = tool.description,
                onClick = { }
            )
            ModernListItem(
                title = "类型",
                subtitle = tool.toolType,
                onClick = { }
            )
            ModernListItem(
                title = "调用次数",
                subtitle = "${tool.callCount}",
                onClick = { }
            )
            if (!tool.path.isNullOrBlank()) {
                ModernListItem(
                    title = "路径",
                    subtitle = tool.path!!,
                    onClick = { }
                )
            }
        }

        Spacer(modifier = Modifier.height(16.dp))

        // Toggle
        ModernSwitchItem(
            icon = Icons.Default.Build,
            title = if (tool.enabled) "已启用" else "已禁用",
            subtitle = "切换工具启用状态",
            checked = tool.enabled,
            onCheckedChange = onToggle
        )

        Spacer(modifier = Modifier.height(32.dp))
    }
}
