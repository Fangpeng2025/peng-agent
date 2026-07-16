package com.peng.agent.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Api
import androidx.compose.material.icons.filled.Memory
import androidx.compose.material.icons.filled.Palette
import androidx.compose.material.icons.filled.Person
import androidx.compose.material.icons.filled.Security
import androidx.compose.material.icons.filled.Speed
import androidx.compose.material.icons.filled.SmartToy
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.peng.agent.client.BackendClient
import com.peng.agent.ui.components.ModernGroupCard
import com.peng.agent.ui.components.ModernListItem
import com.peng.agent.ui.components.ModernSwitchItem

@Composable
fun SettingsScreen(
    client: BackendClient,
    modifier: Modifier = Modifier
) {
    val config by client.config.collectAsState()
    val tools by client.tools.collectAsState()

    var editConfig by remember { mutableStateOf(config) }

    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 16.dp, vertical = 8.dp)
    ) {
        // Title
        Text(
            text = "设置",
            style = MaterialTheme.typography.headlineLarge,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.padding(bottom = 16.dp)
        )

        // ── AI模型配置 ──
        ModernGroupCard(title = "AI模型配置") {
            ModernListItem(
                icon = Icons.Default.SmartToy,
                title = "模型",
                subtitle = editConfig.model,
                onClick = { /* TODO: show model picker */ }
            )
            ModernListItem(
                icon = Icons.Default.Api,
                title = "API地址",
                subtitle = editConfig.apiBase,
                onClick = { /* TODO: edit api base */ }
            )
            ModernListItem(
                icon = Icons.Default.Security,
                title = "API密钥",
                subtitle = if (editConfig.apiKey.isNotEmpty()) "${editConfig.apiKey.take(8)}..." else "未设置",
                onClick = { /* TODO: edit api key */ }
            )
        }

        Spacer(modifier = Modifier.height(16.dp))

        // ── 子Agent配置 ──
        ModernGroupCard(title = "子Agent配置") {
            ModernListItem(
                icon = Icons.Default.SmartToy,
                title = "Worker模型",
                subtitle = editConfig.workerModel.ifEmpty { "使用主模型" },
                onClick = { /* TODO: edit */ }
            )
            ModernListItem(
                icon = Icons.Default.Api,
                title = "Worker API地址",
                subtitle = editConfig.workerApiBase.ifEmpty { "使用主API地址" },
                onClick = { /* TODO: edit */ }
            )
        }

        Spacer(modifier = Modifier.height(16.dp))

        // ── 生成参数 ──
        ModernGroupCard(title = "生成参数") {
            ModernListItem(
                icon = Icons.Default.Memory,
                title = "最大Token数",
                subtitle = "${editConfig.maxTokens}",
                onClick = { /* TODO: edit */ }
            )
            ModernListItem(
                icon = Icons.Default.Speed,
                title = "温度",
                subtitle = "${editConfig.temperature}",
                onClick = { /* TODO: edit */ }
            )
            ModernListItem(
                icon = Icons.Default.Speed,
                title = "Top-P",
                subtitle = "${editConfig.topP}",
                onClick = { /* TODO: edit */ }
            )
            ModernListItem(
                icon = Icons.Default.Speed,
                title = "最大轮数",
                subtitle = "${editConfig.maxTurns}",
                onClick = { /* TODO: edit */ }
            )
            ModernListItem(
                icon = Icons.Default.Speed,
                title = "工具超时(秒)",
                subtitle = "${editConfig.toolTimeoutSecs}",
                onClick = { /* TODO: edit */ }
            )
        }

        Spacer(modifier = Modifier.height(16.dp))

        // ── 个性化 ──
        ModernGroupCard(title = "个性化") {
            ModernListItem(
                icon = Icons.Default.Person,
                title = "用户名",
                subtitle = editConfig.userName,
                onClick = { /* TODO: edit */ }
            )
            ModernListItem(
                icon = Icons.Default.Palette,
                title = "回复风格",
                subtitle = editConfig.userStyle,
                onClick = { /* TODO: edit */ }
            )
            ModernSwitchItem(
                icon = Icons.Default.Speed,
                title = "并行执行工具",
                subtitle = "允许同时执行多个工具",
                checked = editConfig.toolExecutionParallel,
                onCheckedChange = { checked ->
                    editConfig = editConfig.copy(toolExecutionParallel = checked)
                    client.setConfig(editConfig)
                }
            )
            ModernSwitchItem(
                icon = Icons.Default.Security,
                title = "出错时中止",
                subtitle = "工具执行出错时中止对话",
                checked = editConfig.abortOnError,
                onCheckedChange = { checked ->
                    editConfig = editConfig.copy(abortOnError = checked)
                    client.setConfig(editConfig)
                }
            )
        }

        Spacer(modifier = Modifier.height(16.dp))

        // ── 视觉模型 ──
        ModernGroupCard(title = "视觉模型") {
            ModernListItem(
                icon = Icons.Default.SmartToy,
                title = "视觉模型",
                subtitle = editConfig.visionModel,
                onClick = { /* TODO: edit */ }
            )
            ModernListItem(
                icon = Icons.Default.Api,
                title = "视觉API地址",
                subtitle = editConfig.visionApiBase,
                onClick = { /* TODO: edit */ }
            )
        }

        Spacer(modifier = Modifier.height(32.dp))
    }
}
