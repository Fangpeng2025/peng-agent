package com.peng.agent.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.ToggleOff
import androidx.compose.material.icons.filled.ToggleOn
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.peng.agent.client.SkillInfo
import com.peng.agent.ui.components.ModernCard
import com.peng.agent.ui.components.ModernGroupCard
import com.peng.agent.ui.components.ModernListItem
import com.peng.agent.ui.components.ModernSwitchItem
import com.peng.agent.ui.theme.BrandPrimary

@Composable
fun SkillDetailPage(
    skill: SkillInfo,
    onBack: () -> Unit,
    onToggle: (Boolean) -> Unit,
    onDelete: () -> Unit,
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
                Text(
                    text = skill.name ?: "技能详情",
                    style = MaterialTheme.typography.titleLarge,
                    color = MaterialTheme.colorScheme.onSurface,
                    modifier = Modifier.weight(1f)
                )
                IconButton(onClick = { onDelete() }) {
                    Icon(
                        imageVector = Icons.Default.Delete,
                        contentDescription = "删除",
                        tint = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
        }

        Spacer(modifier = Modifier.height(16.dp))

        // Skill info
        ModernGroupCard(title = "基本信息") {
            ModernListItem(
                title = "名称",
                subtitle = skill.name ?: "",
                onClick = { }
            )
            ModernListItem(
                title = "版本",
                subtitle = skill.version ?: "1.0.0",
                onClick = { }
            )
            if (!skill.author.isNullOrBlank()) {
                ModernListItem(
                    title = "作者",
                    subtitle = skill.author!!,
                    onClick = { }
                )
            }
            if (!skill.description.isNullOrBlank()) {
                ModernListItem(
                    title = "描述",
                    subtitle = skill.description!!,
                    onClick = { }
                )
            }
            ModernListItem(
                title = "来源",
                subtitle = skill.source ?: "user",
                onClick = { }
            )
            ModernListItem(
                title = "类型",
                subtitle = skill.skillType ?: "document",
                onClick = { }
            )
            if (!skill.tags.isNullOrEmpty()) {
                ModernListItem(
                    title = "标签",
                    subtitle = skill.tags!!.joinToString(", "),
                    onClick = { }
                )
            }
            ModernListItem(
                title = "调用次数",
                subtitle = "${skill.callCount}",
                onClick = { }
            )
        }

        Spacer(modifier = Modifier.height(16.dp))

        // Toggle
        ModernSwitchItem(
            icon = if (skill.enabled) Icons.Default.ToggleOn else Icons.Default.ToggleOff,
            title = if (skill.enabled) "已启用" else "已禁用",
            subtitle = "切换技能启用状态",
            checked = skill.enabled,
            onCheckedChange = onToggle
        )

        Spacer(modifier = Modifier.height(16.dp))

        // Content preview
        if (!skill.content.isNullOrBlank()) {
            ModernGroupCard(title = "内容预览") {
                ModernCard(onClick = null, modifier = Modifier.fillMaxWidth()) {
                    Text(
                        text = skill.content!!.take(500),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(12.dp)
                    )
                }
            }
        }

        Spacer(modifier = Modifier.height(32.dp))
    }
}
