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
import androidx.compose.material.icons.filled.Schedule
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
fun CronScreen(
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
            text = "定时任务",
            style = MaterialTheme.typography.headlineLarge,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.padding(bottom = 16.dp)
        )

        // TODO: Implement cron task management with BackendNative
        ModernEmptyState(
            icon = Icons.Default.Schedule,
            title = "定时任务",
            description = "创建和管理定时执行的任务。\n此功能正在开发中。",
            actionLabel = "创建任务",
            onAction = { /* TODO */ }
        )

        Spacer(modifier = Modifier.height(16.dp))

        ModernGroupCard(title = "示例任务") {
            ModernListItem(
                icon = Icons.Default.Schedule,
                title = "每日摘要",
                subtitle = "每天 8:00 生成前一天会话摘要",
                onClick = { /* TODO */ }
            )
            ModernListItem(
                icon = Icons.Default.Add,
                title = "知识库更新",
                subtitle = "每周一 10:00 更新知识库",
                onClick = { /* TODO */ }
            )
        }
    }
}
