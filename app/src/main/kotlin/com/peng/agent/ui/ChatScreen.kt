package com.peng.agent.ui

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ChatBubbleOutline
import androidx.compose.material.icons.filled.Send
import androidx.compose.material.icons.filled.Stop
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.peng.agent.client.BackendClient
import com.peng.agent.client.ChatMessage
import com.peng.agent.client.StepType
import com.peng.agent.client.ToolCall
import com.peng.agent.ui.components.ModernAiBubble
import com.peng.agent.ui.components.ModernEmptyState
import com.peng.agent.ui.components.ModernUserBubble
import com.peng.agent.ui.components.SkeletonMessageCard
import com.peng.agent.ui.theme.BrandContainer
import com.peng.agent.ui.theme.BrandPrimary
import com.peng.agent.ui.theme.BrandPrimaryDark
import com.peng.agent.util.ToolNameCn
import kotlinx.coroutines.launch

// ═══════════════════════════════════════════════════════════════════════
//  ChatScreen — 集成会话列表 + 聊天界面
// ═══════════════════════════════════════════════════════════════════════

@Composable
fun ChatScreen(
    client: BackendClient,
    modifier: Modifier = Modifier
) {
    val currentSessionId by client.currentSessionId.collectAsState()
    val sessions by client.sessions.collectAsState()

    AnimatedContent(
        targetState = currentSessionId,
        transitionSpec = { fadeIn() togetherWith fadeOut() },
        label = "chat_session"
    ) { sessionId ->
        if (sessionId.isBlank()) {
            // ── 会话列表 ─────────────────────────────────────────
            SessionListPanel(
                client = client,
                onSessionClick = { client.setCurrentSession(it) }
            )
        } else {
            // ── 聊天界面 ─────────────────────────────────────────
            ChatPanel(
                client = client,
                onBack = { client.setCurrentSession("") }
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  SessionListPanel — 会话列表
// ═══════════════════════════════════════════════════════════════════════

@Composable
private fun SessionListPanel(
    client: BackendClient,
    onSessionClick: (String) -> Unit
) {
    val sessions by client.sessions.collectAsState()

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(horizontal = 16.dp)
    ) {
        // Header
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(vertical = 16.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = "会话",
                style = MaterialTheme.typography.headlineLarge,
                color = MaterialTheme.colorScheme.onBackground,
                fontWeight = FontWeight.Bold
            )
            Spacer(modifier = Modifier.weight(1f))
            IconButton(
                onClick = {
                    val session = client.newSession()
                    onSessionClick(session.id)
                }
            ) {
                Icon(
                    imageVector = Icons.Default.Add,
                    contentDescription = "新建会话",
                    tint = BrandPrimary
                )
            }
        }

        if (sessions.isEmpty()) {
            ModernEmptyState(
                icon = Icons.Default.ChatBubbleOutline,
                title = "暂无会话",
                description = "点击右上角 + 开始新对话",
                modifier = Modifier.weight(1f)
            )
        } else {
            LazyColumn(
                verticalArrangement = Arrangement.spacedBy(8.dp),
                modifier = Modifier.fillMaxSize()
            ) {
                items(sessions, key = { it.id }) { session ->
                    Card(
                        onClick = { onSessionClick(session.id) },
                        modifier = Modifier.fillMaxWidth(),
                        shape = RoundedCornerShape(12.dp),
                        colors = CardDefaults.cardColors(
                            containerColor = MaterialTheme.colorScheme.surface
                        ),
                        elevation = CardDefaults.cardElevation(defaultElevation = 0.dp),
                        border = androidx.compose.foundation.BorderStroke(
                            1.dp,
                            MaterialTheme.colorScheme.outlineVariant
                        )
                    ) {
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(16.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Icon(
                                imageVector = Icons.Default.ChatBubbleOutline,
                                contentDescription = null,
                                modifier = Modifier.size(20.dp),
                                tint = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                            Spacer(modifier = Modifier.width(12.dp))
                            Column(modifier = Modifier.weight(1f)) {
                                Text(
                                    text = session.title,
                                    style = MaterialTheme.typography.bodyLarge,
                                    color = MaterialTheme.colorScheme.onSurface,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis
                                )
                                Text(
                                    text = session.id.take(8),
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  ChatPanel — 聊天界面
// ═══════════════════════════════════════════════════════════════════════

@Composable
private fun ChatPanel(
    client: BackendClient,
    onBack: () -> Unit
) {
    val messages by client.currentMessages.collectAsState()
    val isStreaming by client.isStreaming.collectAsState()
    val contextUsage by client.contextUsage.collectAsState()
    val config by client.config.collectAsState()
    val sessionId by client.currentSessionId.collectAsState()

    var inputText by remember { mutableStateOf("") }
    val listState = rememberLazyListState()
    val scope = rememberCoroutineScope()

    Column(modifier = Modifier.fillMaxSize()) {
        // Top bar with back + session title
        Surface(
            modifier = Modifier.fillMaxWidth(),
            color = MaterialTheme.colorScheme.surface,
            shadowElevation = 1.dp
        ) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 8.dp, vertical = 4.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                TextButton(onClick = onBack) {
                    Text(
                        text = "← 返回",
                        style = MaterialTheme.typography.bodyMedium,
                        color = BrandPrimary
                    )
                }
                Spacer(modifier = Modifier.width(8.dp))
                Text(
                    text = sessionId.take(8),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }

        // Context usage indicator
        val usagePercent = contextUsage.usagePercent(config.contextWindow)
        if (usagePercent > 0) {
            LinearProgressIndicator(
                progress = { usagePercent / 100f },
                modifier = Modifier
                    .fillMaxWidth()
                    .height(2.dp)
                    .clip(RoundedCornerShape(1.dp)),
                color = if (usagePercent > 80) MaterialTheme.colorScheme.error else BrandPrimary,
                trackColor = MaterialTheme.colorScheme.surfaceVariant
            )
        }

        // Messages list
        LazyColumn(
            state = listState,
            modifier = Modifier
                .weight(1f)
                .fillMaxWidth()
                .padding(horizontal = 12.dp),
            verticalArrangement = Arrangement.spacedBy(4.dp)
        ) {
            items(messages, key = { it.id }) { message ->
                MessageBubble(message = message, client = client)
            }
            if (isStreaming && messages.isEmpty()) {
                item { SkeletonMessageCard(isUser = false) }
            }
        }

        // Auto-scroll
        if (messages.isNotEmpty()) {
            LaunchedEffect(messages.last().id) {
                listState.animateScrollToItem(messages.size - 1)
            }
        }

        // Input bar
        Surface(
            modifier = Modifier.fillMaxWidth(),
            color = MaterialTheme.colorScheme.surface,
            shadowElevation = 2.dp
        ) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 12.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                OutlinedTextField(
                    value = inputText,
                    onValueChange = { inputText = it },
                    modifier = Modifier.weight(1f),
                    placeholder = {
                        Text(
                            "输入消息...",
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    },
                    shape = RoundedCornerShape(24.dp),
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedBorderColor = BrandPrimary,
                        unfocusedBorderColor = MaterialTheme.colorScheme.outline,
                        cursorColor = BrandPrimary,
                        focusedContainerColor = MaterialTheme.colorScheme.surface,
                        unfocusedContainerColor = MaterialTheme.colorScheme.surfaceVariant
                    ),
                    maxLines = 4
                )
                Spacer(modifier = Modifier.size(8.dp))
                IconButton(
                    onClick = {
                        if (isStreaming) {
                            client.abortStream()
                        } else if (inputText.isNotBlank()) {
                            val msg = inputText.trim()
                            inputText = ""
                            scope.launch {
                                client.streamChat(
                                    message = msg,
                                    history = messages,
                                    onChunk = { },
                                    onToolCall = { }
                                )
                            }
                        }
                    },
                    enabled = inputText.isNotBlank() || isStreaming
                ) {
                    Icon(
                        imageVector = if (isStreaming) Icons.Default.Stop else Icons.Default.Send,
                        contentDescription = if (isStreaming) "停止" else "发送",
                        tint = if (isStreaming) MaterialTheme.colorScheme.error else BrandPrimary,
                        modifier = Modifier.size(24.dp)
                    )
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  MessageBubble
// ═══════════════════════════════════════════════════════════════════════

@Composable
fun MessageBubble(
    message: ChatMessage,
    client: BackendClient
) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 2.dp)
    ) {
        if (message.isUser) {
            ModernUserBubble(text = message.text)
        } else {
            // Tool calls
            if (message.toolCalls.isNotEmpty()) {
                message.toolCalls.forEach { toolCall ->
                    ToolCallCard(toolCall = toolCall)
                }
            }

            // Thinking steps
            if (message.steps.isNotEmpty()) {
                message.steps
                    .filter { it.type == StepType.THINKING || it.text.isNotBlank() }
                    .take(3)
                    .forEach { step ->
                        Text(
                            text = step.text.take(200),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.padding(horizontal = 16.dp, vertical = 2.dp)
                        )
                    }
            }

            // AI text
            if (message.text.isNotBlank()) {
                ModernAiBubble(text = message.text)
            }

            // Error
            if (message.errorMessage != null) {
                Text(
                    text = "❌ ${message.errorMessage}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.padding(horizontal = 16.dp)
                )
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  ToolCallCard
// ═══════════════════════════════════════════════════════════════════════

@Composable
fun ToolCallCard(toolCall: ToolCall) {
    val toolNameCn = ToolNameCn.getCnName(toolCall.name)
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 4.dp),
        shape = RoundedCornerShape(12.dp),
        colors = CardDefaults.cardColors(
            containerColor = BrandContainer
        ),
        elevation = CardDefaults.cardElevation(defaultElevation = 0.dp)
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            Text(
                text = "🔧 $toolNameCn (${toolCall.name})",
                style = MaterialTheme.typography.labelMedium,
                color = BrandPrimaryDark
            )
            if (toolCall.arguments.isNotBlank() && toolCall.arguments != "{}") {
                Text(
                    text = toolCall.arguments.take(100),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 2
                )
            }
            if (toolCall.result != null) {
                Text(
                    text = "→ ${toolCall.result!!.take(200)}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 3
                )
            }
        }
    }
}
