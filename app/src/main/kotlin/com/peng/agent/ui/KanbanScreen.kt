package com.peng.agent.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.RadioButtonUnchecked
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.peng.agent.client.BackendClient
import com.peng.agent.ui.components.ModernCompactCard
import com.peng.agent.ui.components.ModernEmptyState
import com.peng.agent.ui.theme.BrandPrimary

data class KanbanTask(
    val id: String,
    val title: String,
    val description: String = "",
    val status: String = "todo", // todo, doing, done
    val priority: String = "medium" // low, medium, high
)

@Composable
fun KanbanScreen(
    client: BackendClient,
    modifier: Modifier = Modifier
) {
    var tasks by remember {
        mutableStateOf(
            listOf(
                KanbanTask("1", "实现搜索功能", "添加全文搜索", "doing", "high"),
                KanbanTask("2", "优化UI性能", "减少重组次数", "todo", "medium"),
                KanbanTask("3", "添加主题切换", "深色/浅色主题", "done", "low")
            )
        )
    }

    Column(
        modifier = modifier
            .fillMaxSize()
            .padding(horizontal = 16.dp, vertical = 8.dp)
    ) {
        Text(
            text = "看板",
            style = MaterialTheme.typography.headlineLarge,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.padding(bottom = 16.dp)
        )

        val todoTasks = tasks.filter { it.status == "todo" }
        val doingTasks = tasks.filter { it.status == "doing" }
        val doneTasks = tasks.filter { it.status == "done" }

        if (tasks.isEmpty()) {
            ModernEmptyState(
                icon = Icons.Default.Add,
                title = "暂无任务",
                description = "添加任务来管理项目进度"
            )
        } else {
            LazyColumn {
                item {
                    TaskSection("待办", todoTasks) { task ->
                        tasks = tasks.map { if (it.id == task.id) it.copy(status = "doing") else it }
                    }
                    Spacer(modifier = Modifier.height(16.dp))
                }
                item {
                    TaskSection("进行中", doingTasks) { task ->
                        tasks = tasks.map { if (it.id == task.id) it.copy(status = "done") else it }
                    }
                    Spacer(modifier = Modifier.height(16.dp))
                }
                item {
                    TaskSection("已完成", doneTasks) { }
                }
            }
        }
    }
}

@Composable
private fun TaskSection(
    title: String,
    tasks: List<KanbanTask>,
    onTaskClick: (KanbanTask) -> Unit
) {
    Text(
        text = "$title (${tasks.size})",
        style = MaterialTheme.typography.titleSmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.padding(bottom = 8.dp)
    )
    tasks.forEach { task ->
        ModernCompactCard(
            icon = if (task.status == "done") Icons.Default.CheckCircle else Icons.Default.RadioButtonUnchecked,
            title = task.title,
            subtitle = task.description,
            onClick = { onTaskClick(task) },
            accentColor = when (task.priority) {
                "high" -> MaterialTheme.colorScheme.error
                "low" -> MaterialTheme.colorScheme.onSurfaceVariant
                else -> BrandPrimary
            }
        )
        Spacer(modifier = Modifier.height(8.dp))
    }
}
