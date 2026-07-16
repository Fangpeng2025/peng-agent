package com.peng.agent.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Extension
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.peng.agent.client.BackendClient
import com.peng.agent.client.SkillInfo
import com.peng.agent.ui.components.ModernEmptyState
import com.peng.agent.ui.components.ModernGroupCard
import com.peng.agent.ui.components.ModernListItem
import com.peng.agent.ui.theme.BrandPrimary

@Composable
fun SkillsScreen(
    client: BackendClient,
    onSkillClick: (SkillInfo) -> Unit,
    modifier: Modifier = Modifier
) {
    val skills by client.skills.collectAsState()
    val enabledSkills = skills.filter { it.enabled }
    val disabledSkills = skills.filter { !it.enabled }

    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 16.dp, vertical = 8.dp)
    ) {
        Text(
            text = "技能",
            style = MaterialTheme.typography.headlineLarge,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.padding(bottom = 16.dp)
        )

        if (skills.isEmpty()) {
            ModernEmptyState(
                icon = Icons.Default.Extension,
                title = "暂无技能",
                description = "安装技能来增强Agent的能力。\n技能定义了Agent的专业知识域和回复风格。",
                actionLabel = "安装技能",
                onAction = { /* TODO */ }
            )
        } else {
            // Enabled skills
            if (enabledSkills.isNotEmpty()) {
                ModernGroupCard(title = "已启用 (${enabledSkills.size})") {
                    enabledSkills.forEach { skill ->
                        ModernListItem(
                            icon = Icons.Default.Extension,
                            title = skill.name ?: "",
                            subtitle = skill.description?.take(50) ?: "",
                            onClick = { onSkillClick(skill) }
                        )
                    }
                }
                Spacer(modifier = Modifier.height(16.dp))
            }

            // Disabled skills
            if (disabledSkills.isNotEmpty()) {
                ModernGroupCard(title = "已禁用 (${disabledSkills.size})") {
                    disabledSkills.forEach { skill ->
                        ModernListItem(
                            icon = Icons.Default.Extension,
                            title = skill.name ?: "",
                            subtitle = skill.description?.take(50) ?: "",
                            onClick = { onSkillClick(skill) }
                        )
                    }
                }
                Spacer(modifier = Modifier.height(16.dp))
            }

            // Stats
            ModernGroupCard(title = "统计") {
                ModernListItem(
                    title = "总技能数",
                    subtitle = "${skills.size}",
                    onClick = { }
                )
                ModernListItem(
                    title = "已启用",
                    subtitle = "${enabledSkills.size}",
                    onClick = { }
                )
                ModernListItem(
                    title = "总调用次数",
                    subtitle = "${skills.sumOf { it.callCount }}",
                    onClick = { }
                )
            }
        }

        Spacer(modifier = Modifier.height(32.dp))
    }
}
