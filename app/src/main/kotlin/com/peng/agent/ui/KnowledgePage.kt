package com.peng.agent.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Book
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Sync
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

@Composable
fun KnowledgePage(
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
            text = "知识库",
            style = MaterialTheme.typography.headlineLarge,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.padding(bottom = 16.dp)
        )

        ModernGroupCard(title = "知识库管理") {
            ModernListItem(
                icon = Icons.Default.Search,
                title = "搜索限制",
                subtitle = "每次搜索 ${config.knowledgeSearchLimit} 条文档",
                onClick = { /* TODO */ }
            )
            ModernListItem(
                icon = Icons.Default.Book,
                title = "最大文档数",
                subtitle = if (config.knowledgeMaxDocs > 0) "${config.knowledgeMaxDocs}" else "无限制",
                onClick = { /* TODO */ }
            )
            ModernListItem(
                icon = Icons.Default.Sync,
                title = "自动清理",
                subtitle = if (config.knowledgeAutoCleanup) "已开启 (${config.knowledgeMaxAgeDays}天)" else "已关闭",
                onClick = { /* TODO */ }
            )
            ModernListItem(
                icon = Icons.Default.Delete,
                title = "诊断知识库",
                subtitle = "检查知识库状态",
                onClick = {
                    client.diagnoseKnowledge()
                }
            )
        }

        Spacer(modifier = Modifier.height(16.dp))

        ModernEmptyState(
            icon = Icons.Default.Book,
            title = "知识库",
            description = "上传和管理知识文档，供Agent参考。\n知识文档将在对话时自动检索。",
            actionLabel = "添加知识",
            onAction = { /* TODO */ }
        )
    }
}
